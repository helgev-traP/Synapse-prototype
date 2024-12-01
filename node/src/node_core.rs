use std::{
    ops::Index,
    sync::{Arc, Weak},
};

use tokio::sync::{Mutex, MutexGuard, RwLock};

use crate::socket::{InputTrait, OutputTrait, OutputTree};

use super::{
    channel::{Channel, FrontToNode, NodeToFront},
    err::UpdateInputDefaultError,
    socket::InputGroup,
    types::{NodeId, NodeName, SharedAny, SocketId},
    FrameCount,
};

/// # NodeFn
// todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
#[async_trait::async_trait]
pub trait NodeFn<NodeInputs, MEMORY, OUTPUT>: Send + Sync + 'static
where
    NodeInputs: InputGroup + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    async fn call(
        &self,
        name: &NodeName,
        input: &NodeInputs,
        memory: &mut MEMORY,
        frame: FrameCount,
    ) -> OUTPUT;
}

#[async_trait::async_trait]
impl<NodeInputs, MEMORY, OUTPUT, F> NodeFn<NodeInputs, MEMORY, OUTPUT> for F
where
    NodeInputs: InputGroup + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
    // F: Fn(&NodeName, &NodeInputs, &mut MEMORY, FrameCount) -> OUTPUT + Send + Sync + 'static,
    F: for<'a> Fn(
            &'a NodeName,
            &'a NodeInputs,
            &'a mut MEMORY,
            FrameCount,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = OUTPUT> + Send + 'a>>
        + Send
        + Sync
        + 'static,
{
    async fn call(
        &self,
        name: &NodeName,
        input: &NodeInputs,
        memory: &mut MEMORY,
        frame: FrameCount,
    ) -> OUTPUT {
        self(name, input, memory, frame).await
    }
}

pub struct NodeCore<Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Send + Sync + 'static,
{
    // ids
    id: NodeId,
    name: Arc<Mutex<String>>,
    // input
    input: Arc<Mutex<Option<Inputs>>>,
    // memory
    memory: Arc<Mutex<Memory>>,
    // main process
    main_process: Box<dyn NodeFn<Inputs, Memory, ProcessOutput>>,
    // cache
    cache: Mutex<Cache<ProcessOutput>>,
    // output
    output: Arc<Mutex<Option<OutputTree>>>,
    // com
    com_to_frontend: Arc<Mutex<Option<Channel<NodeToFront, FrontToNode>>>>,
}

/// constructors
impl<Inputs, Memory, ProcessOutput> NodeCore<Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Send + Sync + 'static,
{
    pub fn new(
        name: &str,
        memory: Memory,
        main_process: Box<dyn NodeFn<Inputs, Memory, ProcessOutput>>,
    ) -> Self {
        NodeCore {
            id: NodeId::new(),
            name: Arc::new(Mutex::new(name.to_string())),
            input: Arc::new(Mutex::new(None)),
            memory: Arc::new(tokio::sync::Mutex::new(memory)),
            main_process,
            cache: Mutex::new(Cache::new(1)), // キャッシュサイズはNodeFieldにpushする前に統一するので、初期値はなんでもいい
            output: Arc::new(Mutex::new(None)),
            com_to_frontend: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_input(&self, input: Inputs) {
        *self.input.lock().await = Some(input);
    }

    pub async fn set_output(&self, output: OutputTree) {
        *self.output.lock().await = Some(output);
    }
}

/// main process
impl<Inputs, Memory, ProcessOutput> NodeCore<Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Send + Sync + 'static,
{
    // call by output socket.
    pub async fn call(&self, frame: FrameCount) -> Arc<ProcessOutput> {
        let mut cache = self.cache.lock().await;

        if let Ok(data) = cache.search(frame) {
            return data.data.clone();
        }

        let name = self.name.lock().await;
        let input = self.input.lock().await;
        let mut memory = self.memory.lock().await;

        let output = self
            .main_process
            .call(
                &*name,
                &*input.as_ref().expect("input is not set."),
                &mut *memory,
                frame,
            )
            .await;

        cache.push(frame, output);

        cache.get_first().as_ref().unwrap().data.clone()
    }
}

/// others
/// cache, coms, updating default value of input.
impl<Inputs, Memory, ProcessOutput> NodeCore<Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Send + Sync + 'static,
{
    // channel to frontend
    pub async fn set_com_to_frontend(&self, channel: Channel<NodeToFront, FrontToNode>) {
        let mut com_to_frontend = self.com_to_frontend.lock().await;
        *com_to_frontend = Some(channel);
    }

    // only for debug build
    #[cfg(debug_assertions)]
    pub async fn debug_get_cache_top(&self) -> Option<Arc<ProcessOutput>> {
        self.cache
            .lock()
            .await
            .get_first()
            .as_ref()
            .map(|data| data.data.clone())
    }
}

#[async_trait::async_trait]
impl<Inputs, Memory, ProcessOutput> NodeCoreCommon for NodeCore<Inputs, Memory, ProcessOutput>
where
    Inputs: InputGroup + Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    ProcessOutput: Send + Sync + 'static,
{
    fn get_id(&self) -> NodeId {
        self.id
    }

    async fn get_name(&self) -> MutexGuard<'_, String> {
        self.name.lock().await
    }

    async fn set_name(&self, name: String) {
        *self.name.lock().await = name;
    }

    async fn change_cache_depth(&self, new_cache_size: usize) {
        let number_of_output = self
            .output
            .lock()
            .await
            .as_ref()
            .expect("input is not set.")
            .size();
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
        self.cache.lock().await.len()
            / self
                .output
                .lock()
                .await
                .as_ref()
                .expect("input is not set.")
                .size()
    }

    async fn cache_size(&self) -> usize {
        self.cache.lock().await.len()
    }

    async fn get_input_socket(&self, socket_id: SocketId) -> Option<Weak<dyn InputTrait>> {
        self.input
            .lock()
            .await
            .as_ref()
            .expect("input is not set.")
            .get_socket(socket_id)
            .await
    }

    async fn get_output_socket(&self, socket_id: SocketId) -> Option<Weak<dyn OutputTrait>> {
        self.output
            .lock()
            .await
            .as_ref()
            .expect("input is not set.")
            .get_socket(socket_id)
    }

    async fn update_input_default(
        &self,
        input_socket_id: SocketId,
        default: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError> {
        let input = self.input.lock().await;
        match input
            .as_ref()
            .expect("input is not set.")
            .get_socket(input_socket_id)
            .await
        {
            Some(socket) => {
                socket.upgrade().unwrap().set_default_value(default).await?;
                // clear cache
                self.cache.lock().await.clear();
                self.output
                    .lock()
                    .await
                    .as_ref()
                    .expect("input is not set.")
                    .clear_cache()
                    .await;
                Ok(())
            }
            None => Err(UpdateInputDefaultError::SocketIdNotFound(default)),
        }
    }

    async fn call(&self, frame: FrameCount) -> Arc<SharedAny> {
        self.call(frame).await
    }

    async fn play(&self, frame: FrameCount) {
        todo!()
    }
}

// --- NodeCoreCommon ---
// handle Nodes in NodeField uniformly.
#[async_trait::async_trait]
pub trait NodeCoreCommon: Send + Sync {
    // getters and setters
    fn get_id(&self) -> NodeId;
    async fn get_name<'a>(&'a self) -> MutexGuard<'a, String>;
    async fn set_name(&self, name: String);
    // cache
    async fn change_cache_depth(&self, new_cache_size: usize);
    async fn clear_cache(&self);
    async fn cache_depth(&self) -> usize;
    async fn cache_size(&self) -> usize;
    // get input/output socket to: connect, disconnect
    async fn get_input_socket(&self, socket_id: SocketId) -> Option<Weak<dyn InputTrait>>;
    async fn get_output_socket(&self, socket_id: SocketId) -> Option<Weak<dyn OutputTrait>>;
    // update default value of input
    async fn update_input_default(
        &self,
        input_socket_id: SocketId,
        default: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError>;
    // calling one frame
    async fn call(&self, frame: FrameCount) -> Arc<SharedAny>;
    // main playing process(play)
    async fn play(&self, frame: FrameCount);
}

// --- Cache ---

#[derive(Debug)]
struct CacheData<T> {
    frame: FrameCount,
    data: Arc<T>,
}

impl<T> Clone for CacheData<T> {
    fn clone(&self) -> Self {
        CacheData {
            frame: self.frame,
            data: self.data.clone(),
        }
    }
}

struct Cache<T> {
    ring_buffer: Vec<Option<CacheData<T>>>,
    buffer_size: usize,
    buffer_index: usize,
}

// index access
impl<T> Index<usize> for Cache<T> {
    type Output = Option<CacheData<T>>;

    fn index(&self, index: usize) -> &Self::Output {
        return &self.ring_buffer[(self.buffer_index + index) % self.buffer_size];
    }
}

impl<T> Cache<T> {
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

    pub fn push(&mut self, frame: FrameCount, data: T) {
        // Shift the index forward to push data.
        self.buffer_index = (self.buffer_index + self.buffer_size - 1) % self.buffer_size;
        self.ring_buffer[self.buffer_index] = Some(CacheData {
            frame,
            data: Arc::new(data),
        });
    }

    pub fn get_first(&self) -> &Option<CacheData<T>> {
        &self[0]
    }

    pub fn search(&mut self, frame: FrameCount) -> Result<&CacheData<T>, ()> {
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
            cache.push(i, i);
        }
        // 始めから、 9, 8, 7, 6, 5, 4, 3, 2, 1, 0
        // first: 9
        assert_eq!(cache.get_first().as_ref().unwrap().frame, 9);
        // search 5. then 5 become the first.
        // from begin: 5, 9, 8, 7, 6, 4, 3, 2, 1, 0
        assert_eq!(cache.search(5).unwrap().frame, 5);
        // first: 5
        assert_eq!(cache.get_first().as_ref().unwrap().frame, 5);
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
        Box::pin(async move { todo!() })
    }

    fn pickup(output: &i64) -> Box<SharedAny> {
        todo!()
    }

    fn get_node(name: String, default: i64) -> (SocketId, Box<dyn NodeCoreCommon>, SocketId) {
        todo!()
    }

    #[tokio::test]
    async fn node_core_communicate_test() {}
}
