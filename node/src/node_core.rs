use std::{
    collections::HashSet,
    ops::Index,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use super::{
    channel::{Channel, FrontToNode, InputChannel, NodeOrder, NodeToFront, OutputChannel},
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, UpdateInputDefaultError,
    },
    socket::{InputTree, OutputTree},
    types::{NodeId, NodeName, SharedAny, SocketId},
    FrameCount,
};

/// # NodeFn
// todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
#[async_trait::async_trait]
pub trait NodeFn<OUTPUT, MEMORY>: Send + Sync + 'static
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    async fn call(
        &self,
        input: &Arc<tokio::sync::Mutex<InputTree>>,
        memory: &mut MEMORY,
        frame: FrameCount,
        id: &NodeId,
        name: &NodeName,
    ) -> OUTPUT;
}

#[async_trait::async_trait]
impl<OUTPUT, MEMORY, F> NodeFn<OUTPUT, MEMORY> for F
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    F: Fn(
            Arc<tokio::sync::Mutex<InputTree>>,
            &mut MEMORY,
            FrameCount,
            &NodeId,
            &NodeName,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = OUTPUT> + Send>>
        + Send
        + Sync
        + 'static,
{
    async fn call(
        &self,
        input: &Arc<tokio::sync::Mutex<InputTree>>,
        memory: &mut MEMORY,
        frame: FrameCount,
        id: &NodeId,
        name: &NodeName,
    ) -> OUTPUT {
        self(input.clone(), memory, frame, id, name).await
    }
}

pub struct NodeCore<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    // ids
    id: NodeId,
    name: String,
    // input
    input: Arc<tokio::sync::Mutex<InputTree>>,
    // memory
    memory: Arc<tokio::sync::Mutex<MEMORY>>,
    // main process
    main_process: Box<dyn NodeFn<OUTPUT, MEMORY>>,
    // cache
    cache: Cache<OUTPUT>,
    // output
    output: Arc<tokio::sync::Mutex<OutputTree<OUTPUT>>>,
    // com
    com_to_frontend: Arc<Mutex<Option<Channel<NodeToFront, FrontToNode>>>>,
}

/// constructors
impl<OUTPUT, MEMORY> NodeCore<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    pub fn new(
        name: String,
        input: InputTree,
        memory: MEMORY,
        main_process: Box<dyn NodeFn<OUTPUT, MEMORY>>,
        output: OutputTree<OUTPUT>,
    ) -> Self {
        NodeCore {
            id: NodeId::new(),
            name,
            input: Arc::new(tokio::sync::Mutex::new(input)),
            memory: Arc::new(tokio::sync::Mutex::new(memory)),
            main_process,
            cache: Cache::new(1), // キャッシュサイズはNodeFieldにpushする前に統一するので、初期値はなんでもいい
            output: Arc::new(tokio::sync::Mutex::new(output)),
            com_to_frontend: Arc::new(Mutex::new(None)),
        }
    }
}

/// getters and setters
impl<OUTPUT, MEMORY> NodeCore<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    // getters

    fn get_id(&self) -> &NodeId {
        &self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    // setters

    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

/// connect and disconnect
impl<OUTPUT, MEMORY> NodeCore<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    // connect channel

    pub async fn connect_output(
        &self,
        channel: OutputChannel,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        match self.output.lock().await.get_from_id(socket_id).await {
            Ok(socket) => {
                socket
                    .lock()
                    .await
                    .connect(channel, downstream_socket_id)
                    .await
            }
            Err(_) => Err(NodeConnectError::SocketIdNotFound),
        }
    }

    pub async fn send_connection_checker(
        &self,
        token: Uuid,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectionCheckError> {
        match self.output.lock().await.get_from_id(socket_id).await {
            Ok(socket) => {
                socket
                    .lock()
                    .await
                    .send_connection_checker(token, downstream_socket_id)
                    .await
            }
            Err(_) => Err(NodeConnectionCheckError::SocketIdNotFound),
        }
    }

    // todo
    pub async fn disconnect_output(
        &self,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        match self.output.lock().await.get_from_id(socket_id).await {
            Ok(socket) => socket.lock().await.disconnect(downstream_socket_id).await,
            Err(_) => Err(NodeDisconnectError::SocketIdNotFound),
        }
    }

    pub async fn connect_input(
        &mut self,
        channel: InputChannel,
        socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        match self.input.lock().await.get_from_id(socket_id) {
            Ok(socket) => {
                socket.connect(channel).await?;
                self.cache.clear();
                self.output.lock().await.send_delete_cache().await;
                Ok(())
            }
            Err(_) => Err(NodeConnectError::SocketIdNotFound),
        }
    }

    pub async fn recv_connection_checker(
        &self,
        socket_id: &SocketId,
    ) -> Result<Uuid, NodeConnectionCheckError> {
        match self.input.lock().await.get_from_id(socket_id) {
            Ok(socket) => socket.recv_connection_checker().await,
            Err(_) => Err(NodeConnectionCheckError::SocketIdNotFound),
        }
    }

    pub async fn disconnect_input(
        &mut self,
        socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        match self.input.lock().await.get_from_id(socket_id) {
            Ok(socket) => {
                socket.disconnect()?;
                self.cache.clear();
                self.output.lock().await.send_delete_cache().await;
                Ok(())
            }
            Err(_) => Err(NodeDisconnectError::SocketIdNotFound),
        }
    }

    // get input and output

    // todo OUTPUT を介さずに、フロントで欲しいOutputTreeの情報を得られるような仕組みを作る

    pub fn get_input_mutex(&self) -> Arc<tokio::sync::Mutex<InputTree>> {
        self.input.clone()
    }

    pub fn get_output_mutex(&self) -> Arc<tokio::sync::Mutex<OutputTree<OUTPUT>>> {
        self.output.clone()
    }
}

/// others
/// cache, coms, updating default value of input.
impl<OUTPUT, MEMORY> NodeCore<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    // cache

    pub async fn change_cache_depth(&mut self, new_cache_size: usize) {
        let number_of_output = self.output.lock().await.size();
        self.cache.change_size(new_cache_size * number_of_output);
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub async fn cache_depth(&self) -> usize {
        self.cache.len() / self.output.lock().await.size()
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    // channel to frontend

    pub fn set_com_to_frontend(&self, channel: Channel<NodeToFront, FrontToNode>) {
        let mut com_to_frontend = self.com_to_frontend.lock().expect("");
        *com_to_frontend = Some(channel);
    }

    // update default value of input
    pub async fn update_input_default(
        &mut self,
        input_socket_id: &SocketId,
        default: SharedAny,
    ) -> Result<(), UpdateInputDefaultError> {
        let mut input = self.input.lock().await;
        match input.get_from_id(input_socket_id) {
            Ok(socket) => {
                socket.update_default_value(default)?;
                self.cache.clear();
                self.output.lock().await.send_delete_cache().await;
                Ok(())
            }
            Err(_) => Err(UpdateInputDefaultError::SocketIdNotFound(default)),
        }
    }
}

/// main process
impl<OUTPUT, MEMORY> NodeCore<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    /// Returns a vector of (frame, socket_id, downstream_socket_id).
    async fn search_request(&self) -> Vec<(FrameCount, SocketId, SocketId)> {
        let mut requests = Vec::new();
        let output = self.output.lock().await;
        for _ in 0..output.size() {
            let mut output_socket = output.loop_search().await.lock().await;
            match output_socket.try_recv_request() {
                super::types::TryRecvResult::Order(order) => {
                    for (frame, downstream_socket_id) in order {
                        match frame {
                            NodeOrder::Request { frame } => {
                                requests.push((
                                    frame,
                                    output_socket.get_id().clone(),
                                    downstream_socket_id,
                                ));
                            }
                            NodeOrder::TypeConformed => todo!("TypeConformed"),
                            NodeOrder::TypeRejected => todo!("TypeRejected"),
                        }
                    }
                }
                super::types::TryRecvResult::Empty => {}
            };
        }
        requests
    }

    async fn order_input_and_cache(
        &mut self,
        frame: FrameCount,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) {
        // 2. search cache.
        if self.cache.search(frame, socket_id).is_err() {
            // if cache not found.
            // 3. process
            // execute main process.
            let mut memory = self.memory.lock().await;

            // 4. push to cache.
            let result = self
                .main_process
                .call(&self.input, &mut memory, frame, &self.id, &self.name)
                .await;
            self.cache.push(frame, downstream_socket_id, result);
        }
    }

    async fn send_response(
        &self,
        cache_top: &CacheData<OUTPUT>,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) {
        // 5. send response.
        let output = self.output.lock().await;
        let output_socket = output.get_from_id(socket_id).await.unwrap();

        let data = cache_top.data.clone();

        output_socket
            .lock()
            .await
            .send_response(&*data.lock().await, downstream_socket_id)
            .await
            .unwrap();
    }

    async fn for_each_request(
        &mut self,
        frame: FrameCount,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) {
        // 2. search cache, 3. order input and process, 4. write output to cache.
        self.order_input_and_cache(frame, socket_id, downstream_socket_id)
            .await;

        let cache_top = self.cache.get_first().as_ref().unwrap();

        // 5. send response.
        self.send_response(cache_top, socket_id, downstream_socket_id)
            .await;
    }

    pub async fn main(&mut self) {
        // 0. check cache delete request.
        if self.input.lock().await.check_cache_delete_request().await {
            self.cache.clear();
            self.output.lock().await.send_delete_cache().await;
        }
        // 1. search request.
        let requests = self.search_request().await;

        // for each request.
        for (frame, socket_id, downstream_socket_id) in requests {
            self.for_each_request(frame, &socket_id, &downstream_socket_id)
                .await;
        }
    }

    // todo: change process based on event driven.
    // call by output socket.
    pub async fn call(
        &mut self,
        frame: FrameCount,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> OUTPUT {
        todo!()
    }
}

// --- NodeCoreCommon ---
// handle Nodes in NodeField uniformly.

#[async_trait::async_trait]
pub trait NodeCoreCommon: Send + Sync + 'static {
    fn get_id(&self) -> &NodeId;
    fn get_name(&self) -> &String;
    fn set_name(&mut self, name: String);
    async fn change_cache_depth(&mut self, new_cache_size: usize);
    fn clear_cache(&mut self);
    async fn cache_depth(&self) -> usize;
    fn cache_size(&self) -> usize;
    async fn connect_output(
        &self,
        channel: OutputChannel,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectError>;
    async fn disconnect_output(
        &self,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError>;
    async fn send_connection_checker(
        &self,
        token: Uuid,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectionCheckError>;
    async fn connect_input(
        &mut self,
        channel: InputChannel,
        socket_id: &SocketId,
    ) -> Result<(), NodeConnectError>;
    async fn disconnect_input(&mut self, socket_id: &SocketId) -> Result<(), NodeDisconnectError>;
    async fn recv_connection_checker(
        &self,
        socket_id: &SocketId,
    ) -> Result<Uuid, NodeConnectionCheckError>;
    async fn update_input_default(
        &mut self,
        input_socket_id: &SocketId,
        default: SharedAny,
    ) -> Result<(), UpdateInputDefaultError>;
    async fn main(&mut self);
}

#[async_trait::async_trait]
impl<OUTPUT, MEMORY> NodeCoreCommon for NodeCore<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    fn get_id(&self) -> &NodeId {
        self.get_id()
    }

    fn get_name(&self) -> &String {
        self.get_name()
    }

    fn set_name(&mut self, name: String) {
        self.set_name(name)
    }

    async fn change_cache_depth(&mut self, new_cache_size: usize) {
        self.change_cache_depth(new_cache_size).await
    }

    fn clear_cache(&mut self) {
        self.clear_cache()
    }

    async fn cache_depth(&self) -> usize {
        self.cache_depth().await
    }

    fn cache_size(&self) -> usize {
        self.cache_size()
    }

    async fn connect_output(
        &self,
        channel: OutputChannel,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        self.connect_output(channel, socket_id, downstream_socket_id)
            .await
    }

    async fn disconnect_output(
        &self,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        self.disconnect_output(socket_id, downstream_socket_id)
            .await
    }

    async fn send_connection_checker(
        &self,
        token: Uuid,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectionCheckError> {
        self.send_connection_checker(token, socket_id, downstream_socket_id)
            .await
    }

    async fn connect_input(
        &mut self,
        channel: InputChannel,
        socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        self.connect_input(channel, socket_id).await
    }

    async fn disconnect_input(&mut self, socket_id: &SocketId) -> Result<(), NodeDisconnectError> {
        self.disconnect_input(socket_id).await
    }

    async fn recv_connection_checker(
        &self,
        socket_id: &SocketId,
    ) -> Result<Uuid, NodeConnectionCheckError> {
        self.recv_connection_checker(socket_id).await
    }

    async fn update_input_default(
        &mut self,
        input_socket_id: &SocketId,
        default: SharedAny,
    ) -> Result<(), UpdateInputDefaultError> {
        self.update_input_default(input_socket_id, default).await
    }

    async fn main(&mut self) {
        self.main().await
    }
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
    use super::super::{
        channel::{channel_pair, InputChannel, OutputChannel},
        socket::{Input, InputCommon, OutputSocket},
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
        input: Arc<tokio::sync::Mutex<InputTree>>,
        _: &mut (),
        frame: FrameCount,
        _: &NodeId,
        _: &NodeName,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = i64> + Send + 'a>> {
        let input = input.clone();
        Box::pin(async move {
            tokio::spawn(async move { input.lock().await.reef().get_clone(frame).await })
                .await
                .unwrap()
                .downcast_ref::<i64>()
                .unwrap()
                .clone()
        })
    }

    fn pickup(output: &i64) -> SharedAny {
        Box::new(*output)
    }

    fn get_node(name: String, default: i64) -> (SocketId, Box<dyn NodeCoreCommon>, SocketId) {
        let input = Box::new(Input::new(default, (), Box::new(read)));
        let input_id = *input.get_id();
        let output = OutputSocket::new(Box::new(pickup));
        let output_id = *output.get_id();
        (
            input_id,
            Box::new(NodeCore::new(
                name,
                InputTree::Reef(input),
                (),
                Box::new(node_process),
                OutputTree::new_reef(output),
            )),
            output_id,
        )
    }

    #[tokio::test]
    async fn node_core_communicate_test() {
        let (input_id_a, mut node_a, output_id_a) = get_node("a".to_string(), 1);
        let (input_id_b, mut node_b, output_id_b) = get_node("b".to_string(), 2);

        let (mut ch_a, mut ch_b): (OutputChannel, InputChannel) = channel_pair(1);

        let connect_handle_a = tokio::spawn(async move {
            node_a
                .connect_output(ch_a, &output_id_a, &input_id_b)
                .await
                .unwrap();
        });
        let connect_handle_b = tokio::spawn(async move {
            node_b.connect_input(ch_b, &input_id_b).await.unwrap();
        });

        connect_handle_a.await.unwrap();
        connect_handle_b.await.unwrap();

        // todo
    }
}
