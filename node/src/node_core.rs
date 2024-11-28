use std::{
    collections::HashSet,
    ops::Index,
    sync::{Arc, Weak},
};

use tokio::sync::{Mutex, MutexGuard};

use crate::socket::{InputTrait, OutputTrait, OutputTree};

use super::{
    channel::{Channel, FrontToNode, NodeToFront},
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, UpdateInputDefaultError,
    },
    socket::InputGroup,
    types::{NodeId, NodeName, SharedAny, SocketId},
    FrameCount,
};

/// # NodeFn
// todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
#[async_trait::async_trait]
pub trait NodeFn<'a, NodeInputs, MEMORY, OUTPUT>: Send + Sync + 'static
where
    NodeInputs: InputGroup + Send + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Clone + Send + Sync + 'static,
{
    async fn call(
        &'a self,
        name: &'a NodeName,
        input: tokio::sync::RwLockReadGuard<'a, NodeInputs>,
        memory: &'a mut MEMORY,
        frame: FrameCount,
        downstream_id: &'a NodeId,
    ) -> OUTPUT;
}

#[async_trait::async_trait]
impl<'a, NodeInputs, MEMORY, OUTPUT, F> NodeFn<'a, NodeInputs, MEMORY, OUTPUT> for F
where
    NodeInputs: InputGroup + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Clone + Send + Sync + 'static,
    F: Fn(
            &NodeName,
            tokio::sync::RwLockReadGuard<NodeInputs>,
            &mut MEMORY,
            FrameCount,
            &NodeId,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = OUTPUT> + Send + Sync>>
        + Send
        + Sync
        + 'static,
{
    async fn call(
        &'a self,
        name: &'a NodeName,
        input: tokio::sync::RwLockReadGuard<'a, NodeInputs>,
        memory: &'a mut MEMORY,
        frame: FrameCount,
        downstream_id: &'a NodeId,
    ) -> OUTPUT {
        self(name, input, memory, frame, downstream_id).await
    }
}

pub struct NodeCore<'a, Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Clone + Send + Sync + 'static,
{
    // ids
    id: NodeId,
    name: Mutex<String>,
    // input
    input: Arc<tokio::sync::Mutex<Inputs>>,
    // memory
    memory: Arc<tokio::sync::Mutex<Memory>>,
    // main process
    main_process: Box<dyn NodeFn<'a, Inputs, Memory, ProcessOutput>>,
    // cache
    cache: Mutex<Cache<ProcessOutput>>,
    // output
    output: Arc<tokio::sync::Mutex<OutputTree>>,
    // com
    com_to_frontend: Arc<Mutex<Option<Channel<NodeToFront, FrontToNode>>>>,
}

/// constructors
impl<'a, Inputs, Memory, ProcessOutput> NodeCore<'a, Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Clone + Send + Sync + 'static,
{
    pub fn new(
        name: String,
        input: Inputs,
        memory: Memory,
        main_process: Box<dyn NodeFn<'a, Inputs, Memory, ProcessOutput>>,
        output: OutputTree,
    ) -> Self {
        NodeCore {
            id: NodeId::new(),
            name: Mutex::new(name),
            input: Arc::new(tokio::sync::Mutex::new(input)),
            memory: Arc::new(tokio::sync::Mutex::new(memory)),
            main_process,
            cache: Mutex::new(Cache::new(1)), // キャッシュサイズはNodeFieldにpushする前に統一するので、初期値はなんでもいい
            output: Arc::new(tokio::sync::Mutex::new(output)),
            com_to_frontend: Arc::new(Mutex::new(None)),
        }
    }
}

/// others
/// cache, coms, updating default value of input.
impl<Inputs, Memory, ProcessOutput> NodeCore<'_, Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Clone + Send + Sync + 'static,
{
    // channel to frontend
    pub async fn set_com_to_frontend(&self, channel: Channel<NodeToFront, FrontToNode>) {
        let mut com_to_frontend = self.com_to_frontend.lock().await;
        *com_to_frontend = Some(channel);
    }
}

/// main process
impl<Inputs, Memory, ProcessOutput> NodeCore<'_, Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Clone + Send + Sync + 'static,
{
    // call by output socket.
    pub async fn call(
        &self,
        frame: FrameCount,
        socket_id: SocketId,
        downstream_socket_id: SocketId,
    ) -> &ProcessOutput {
        todo!()
    }
}

#[async_trait::async_trait]
impl<Inputs, Memory, ProcessOutput> NodeCoreCommon for NodeCore<'_, Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Clone + Send + Sync + 'static,
{
    fn get_id(&self) -> &NodeId {
        &self.id
    }

    async fn get_name(&self) -> MutexGuard<'_, String> {
        self.name.lock().await
    }

    async fn set_name(&self, name: String) {
        *self.name.lock().await = name;
    }

    async fn change_cache_depth(&self, new_cache_size: usize) {
        let number_of_output = self.output.lock().await.size();
        self.cache
            .lock()
            .await
            .change_size(new_cache_size * number_of_output);
    }

    async fn clear_cache(&self) {
        self.cache.lock().await.clear();

        // send cache clear to downstream
    }

    async fn cache_depth(&self) -> usize {
        self.cache.lock().await.len() / self.output.lock().await.size()
    }

    async fn cache_size(&self) -> usize {
        self.cache.lock().await.len()
    }

    async fn get_input_socket(&self, socket_id: &SocketId) -> Option<Weak<dyn InputTrait>> {
        self.input.lock().await.get_socket(socket_id).await
    }

    async fn get_output_socket(&self, socket_id: &SocketId) -> Option<Weak<dyn OutputTrait>> {
        self.output.lock().await.get_socket(socket_id)
    }

    async fn update_input_default(
        &self,
        input_socket_id: &SocketId,
        default: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError> {
        let input = self.input.lock().await;
        match input.get_socket(input_socket_id).await {
            Some(socket) => {
                socket.upgrade().unwrap().set_default_value(default).await?;
                // clear cache
                self.cache.lock().await.clear();
                self.output.lock().await.clear_cache();
                Ok(())
            }
            None => Err(UpdateInputDefaultError::SocketIdNotFound(default)),
        }
    }

    async fn play(&self) {
        todo!()
    }
}

// --- NodeCoreCommon ---
// handle Nodes in NodeField uniformly.
#[async_trait::async_trait]
pub trait NodeCoreCommon: Send + Sync {
    // getters and setters
    fn get_id(&self) -> &NodeId;
    async fn get_name<'a>(&'a self) -> MutexGuard<'a, String>;
    async fn set_name(&self, name: String);
    // cache
    async fn change_cache_depth(&self, new_cache_size: usize);
    async fn clear_cache(&self);
    async fn cache_depth(&self) -> usize;
    async fn cache_size(&self) -> usize;
    // get input/output socket to: connect, disconnect
    async fn get_input_socket(&self, socket_id: &SocketId) -> Option<Weak<dyn InputTrait>>;
    async fn get_output_socket(&self, socket_id: &SocketId) -> Option<Weak<dyn OutputTrait>>;
    // update default value of input
    async fn update_input_default(
        &self,
        input_socket_id: &SocketId,
        default: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError>;
    // main playing process(play)
    async fn play(&self);
}

// --- Cache ---

#[derive(Clone, Debug)]
struct CacheData<T: Clone> {
    frame: FrameCount,
    id: HashSet<SocketId>,
    data: Arc<tokio::sync::Mutex<T>>,
}

struct Cache<T: Clone> {
    ring_buffer: Vec<Option<CacheData<T>>>,
    buffer_size: usize,
    buffer_index: usize,
}

impl<T: Clone> Cache<T> {
    pub fn new(buffer_size: usize) -> Self {
        Cache {
            ring_buffer: vec![None; buffer_size],
            buffer_size,
            buffer_index: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.buffer_size
    }

    pub fn push(&mut self, frame: FrameCount, id: &SocketId, data: T) {
        // Shift the index forward to push data.
        self.buffer_index = (self.buffer_index + self.buffer_size - 1) % self.buffer_size;
        let mut id_set = HashSet::new();
        id_set.insert(id.clone());
        self.ring_buffer[self.buffer_index] = Some(CacheData {
            frame,
            id: id_set,
            data: Arc::new(tokio::sync::Mutex::new(data)),
        });
    }

    pub fn get_first(&self) -> &Option<CacheData<T>> {
        &self[0]
    }

    pub fn search(&mut self, frame: FrameCount, id: &SocketId) -> Result<&CacheData<T>, ()> {
        // Search backward from the index.
        let mut found_index: Option<usize> = None;
        for i in 0..self.buffer_size {
            if let Some(data) = &self[i] {
                if data.frame == frame {
                    found_index = Some(i);
                    break;
                }
            }
        }
        match found_index {
            Some(index) => {
                // move to the first
                for i in (0..index).rev() {
                    self.ring_buffer.swap(i, i + 1);
                }
                // insert id
                self.ring_buffer[self.buffer_index]
                    .as_mut()
                    .expect("")
                    .id
                    .insert(id.clone());
                Ok(self[0].as_ref().expect(""))
            }
            None => Err(()),
        }
    }

    pub fn change_size(&mut self, new_buffer_size: usize) {
        // new ring buffer
        let mut new_ring_buffer: Vec<Option<CacheData<T>>> = vec![None; new_buffer_size];
        let copy_size = if self.buffer_size < new_buffer_size {
            self.buffer_size
        } else {
            new_buffer_size
        };
        for i in 0..copy_size {
            new_ring_buffer[i] = self[i].clone();
        }
        // update
        self.ring_buffer = new_ring_buffer;
        self.buffer_size = new_buffer_size;
        self.buffer_index = 0;
    }

    pub fn clear(&mut self) {
        self.ring_buffer = vec![None; self.buffer_size];
        self.buffer_index = 0;
    }
}

// index access
impl<T: Clone> Index<usize> for Cache<T> {
    type Output = Option<CacheData<T>>;

    fn index(&self, index: usize) -> &Self::Output {
        return &self.ring_buffer[(self.buffer_index + index) % self.buffer_size];
    }
}

#[cfg(test)]
mod tests {
    use crate::socket::InputSocket;

    use super::super::{
        channel::{channel_pair, InputChannel, OutputChannel},
        types::{Envelope, SharedAny},
    };

    use super::*;

    #[test]
    fn cache_test() {
        let mut cache = Cache::<i64>::new(10);
        for i in 0..10 {
            cache.push(i, &SocketId::new(), i);
        }
        // 始めから、 9, 8, 7, 6, 5, 4, 3, 2, 1, 0
        // first: 9
        assert_eq!(cache.get_first().as_ref().unwrap().frame, 9);
        // search 5. then 5 become the first.
        // from begin: 5, 9, 8, 7, 6, 4, 3, 2, 1, 0
        assert_eq!(cache.search(5, &SocketId::new()).unwrap().frame, 5);
        // first: 5
        assert_eq!(cache.get_first().as_ref().unwrap().frame, 5);
        // frame 5 is used by another socket_id.
        assert_eq!(cache.get_first().as_ref().expect("").id.len(), 2);
        // change size
        cache.change_size(5);
        // from begin: 5, 9, 8, 7, 6
        assert_eq!(cache.get_first().as_ref().unwrap().frame, 5);
        assert_eq!(cache[0].as_ref().expect("").frame, 5);
        assert_eq!(cache[1].as_ref().expect("").frame, 9);
        assert_eq!(cache[2].as_ref().expect("").frame, 8);
        assert_eq!(cache[3].as_ref().expect("").frame, 7);
        assert_eq!(cache[4].as_ref().expect("").frame, 6);
        // clear
        cache.clear();
        assert_eq!(cache.get_first().is_none(), true);
    }

    fn read(default: &i64, _: &mut (), _: &Envelope, _: FrameCount) -> i64 {
        *default
    }

    fn node_process<'a>(
        input: Arc<tokio::sync::Mutex<dyn InputGroup>>,
        _: &mut (),
        frame: FrameCount,
        _: &NodeId,
        _: &NodeName,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = i64> + Send + 'a>> {
        let input = input.clone();
        Box::pin(async move {
            todo!()
        })
    }

    fn pickup(output: &i64) -> Box<SharedAny> {
        todo!()
    }

    fn get_node(name: String, default: i64) -> (SocketId, Box<dyn NodeCoreCommon>, SocketId) {
        todo!()
    }

    #[tokio::test]
    async fn node_core_communicate_test() {
    }
}
