use uuid::Uuid;

use std::{
    any::Any,
    collections::{HashMap, HashSet},
    iter::from_fn,
    ops::Index,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use super::types::{
    input::{InputSocket, OutputSocket},
    BackendToNode, Channel, FrontendToNode, NodeId, NodeOrder, NodeResponse, NodeToBackend,
    NodeToFrontend, SocketId,
};

use super::{FrameCount, BITS};

enum SocketLayer {
    Vec(Vec<SocketLayer>),
    Socket(SocketId),
    None,
}

impl Index<usize> for SocketLayer {
    type Output = SocketLayer;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            SocketLayer::Vec(vec) => &vec[index],
            _ => panic!(),
        }
    }
}

impl SocketLayer {
    pub fn to_binary(&self) -> Vec<u8> {
        let mut binary = vec![];
        match self {
            SocketLayer::Vec(vec) => {
                binary.push(0);
                binary.extend((vec.len()).to_le_bytes());
                for socket_layer in vec {
                    binary.extend(socket_layer.to_binary());
                }
            }
            SocketLayer::Socket(socket_id) => {
                binary.push(1);
                binary.extend((socket_id.to_string().len()).to_le_bytes());
                binary.extend(socket_id.to_string().as_bytes());
            }
            SocketLayer::None => {
                binary.push(2);
            }
        }
        binary
    }

    pub fn from_binary(binary: &[u8]) -> Result<SocketLayer, ()> {
        let (socket_layer, rest) = SocketLayer::from_binary_recursive(binary)?;
        if rest.len() == 0 {
            Ok(socket_layer)
        } else {
            Err(())
        }
    }

    fn from_binary_recursive(binary: &[u8]) -> Result<(SocketLayer, &[u8]), ()> {
        if binary.len() == 0 {
            return Err(());
        };
        match binary[0] {
            0 => {
                // Vecの長さを確保
                if binary.len() < (BITS / 8) + 1 {
                    return Err(());
                } else {
                    let vec_len =
                        usize::from_le_bytes(binary[1..(BITS / 8) + 1].try_into().unwrap());
                    let mut vec = vec![];
                    let mut binary = &binary[(BITS / 8) + 1..];
                    for _ in 0..vec_len {
                        let (socket_layer, rest) = SocketLayer::from_binary_recursive(binary)?;
                        vec.push(socket_layer);
                        binary = rest;
                    }
                    if binary.len() == 0 {
                        Ok((SocketLayer::Vec(vec), binary))
                    } else {
                        Err(())
                    }
                }
            }
            1 => {
                // 文字列長さを確保
                if binary.len() < (BITS / 8) + 1 {
                    return Err(());
                } else {
                    let socket_id_len = usize::from_le_bytes(
                        binary[(BITS / 8)..(BITS / 8) + 1].try_into().unwrap(),
                    ) as usize;
                    if binary.len() < (BITS / 8) + 1 + socket_id_len {
                        return Err(());
                    } else {
                        let socket_id = String::from_utf8(
                            binary[(BITS / 8) + 1..(BITS / 8) + 1 + socket_id_len].to_vec(),
                        )
                        .unwrap();
                        Ok((
                            SocketLayer::Socket(SocketId::from_string(&socket_id)),
                            &binary[(BITS / 8) + 1 + socket_id_len..],
                        ))
                    }
                }
            }
            2 => Ok((SocketLayer::None, &binary[1..])),
            _ => Err(()),
        }
    }
}

pub struct Data {
    pub data: Box<dyn Any>,
    inputs: SocketLayer,
    outputs: SocketLayer,
}

impl Data {}

/// Node.
///
/// Nodes only can communicate with Data, SocketLayer through NodeCore.
///
/// todo Data, SocketLayerへのアクセス

pub struct NodeCore<OUTPUT: Clone, DATA: 'static, INPUT> {
    // data
    data: Arc<Mutex<Data>>,

    // Node
    node: NodeThread<OUTPUT, DATA, INPUT>,

    // com to Node
    com_node: Channel<BackendToNode, NodeToBackend>,
}

impl<OUTPUT: Clone, DATA: 'static, INPUT> NodeCore<OUTPUT, DATA, INPUT> {
    // get data
    fn get_new_data(node_type: &String) -> Data {
        Data {
            data: Box::new(()),
            inputs: SocketLayer::None,
            outputs: SocketLayer::None,
        }
    }

    fn get_data_from_binary(binary: &[u8]) -> Result<Data, ()> {
        todo!()
    }

    // initialize

    fn new(data: Data) -> Self {
        todo!()
    }

    // boot NodeCore

    fn boot(&self) {
        todo!()
    }
}

/// NodeThread.
///
/// Run the main process, communicate with other nodes.
///
/// // todo 実行の流れを

// make it public for test
// todo make it private
pub struct NodeThread<OUTPUT: Clone, DATA: 'static, INPUT> {
    // # --- shared data ---

    // data shared with NodeCore
    id: Arc<RwLock<NodeId>>,

    name: Arc<RwLock<String>>,

    // data
    input_data: INPUT,

    temporary_data: DATA,

    // cache
    cache: Cache<OUTPUT>,

    // frame
    frame: FrameCount,

    // # --- inner data ---

    // before send order
    frame_order_process: Box<dyn Fn(FrameCount) -> Result<(), ()>>,

    // main process
    main_process: Box<dyn Fn(FrameCount) -> Result<(), ()>>,

    // id of ordered node.
    socket_current_processing_with: SocketId,

    // communication node
    input_sockets: HashMap<SocketId, InputSocket>,
    output_sockets: HashMap<SocketId, OutputSocket>,

    // communication main, client
    com_backend: Channel<NodeToBackend, BackendToNode>,
    com_frontend: Channel<NodeToFrontend, FrontendToNode>,
}

impl<OUTPUT: Clone, DATA: 'static, INPUT> NodeThread<OUTPUT, DATA, INPUT> {
    // com
    fn send_backend(&self, msg: NodeToBackend) {
        self.com_backend.send(msg);
    }

    /// try receive request.
    fn try_recv_request(&mut self) -> Result<(SocketId, FrameCount), ()> {
        for (id, socket) in self.output_sockets.iter_mut() {
            match socket.try_recv_request() {
                Ok(order) => match order {
                    NodeOrder::Request { frame } => {
                        return Ok((id.clone(), frame));
                    }
                    NodeOrder::Disconnect => {
                        socket.disconnect();
                        return Err(());
                    }
                    _ => panic!(),
                },
                Err(_) => {}
            }
        }
        Err(())
    }

    /// send request.
    fn send_request(&self, frame: FrameCount) {
        for (_, socket) in self.input_sockets.iter() {
            socket.send_request(frame);
        }
    }

    /// wait response.
    // todo 今の状態だとどんなメッセージでも受け取って何もしないので中身書く
    fn wait_response(&mut self) {
        for (_, socket) in self.input_sockets.iter_mut() {
            socket.wait_response();
        }
    }

    /// send response.

    fn send_response(&self) {
        let socket = self
            .output_sockets
            .get(&self.socket_current_processing_with)
            .unwrap();
        socket.send_response(self.frame);
    }

    /// control
    pub fn main(&mut self) {
        'main_loop: loop {
            // 1. wait request or shutdown
            'wait_request: loop {
                // shutdown
                if let Ok(msg) = self.com_backend.try_recv() {
                    match msg {
                        BackendToNode::Shutdown => {
                            break 'main_loop;
                        }
                    }
                }
                // try receive request
                if let Ok((id, frame)) = self.try_recv_request() {
                    self.socket_current_processing_with = id;
                    self.frame = frame;
                    break 'wait_request;
                }
            }
            // 2. scan cache
            if self
                .cache
                .search(self.frame, &self.socket_current_processing_with)
                .is_err()
            {
                todo!() // todo 2024/06/24
                        // 3. send request
                        // 4. wait response
                        // 5. main process
                        // 6. push to cache
            }
            // 7. send response
            self.send_response();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CacheData<T: Clone> {
    frame: FrameCount,
    id: HashSet<SocketId>,
    data: T,
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

    pub fn push(&mut self, frame: FrameCount, id: SocketId, data: T) {
        // Shift the index forward to push data.
        self.buffer_index = (self.buffer_index + self.buffer_size - 1) % self.buffer_size;
        let mut id_set = HashSet::new();
        id_set.insert(id);
        self.ring_buffer[self.buffer_index] = Some(CacheData {
            frame,
            id: id_set,
            data,
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
        return &self.ring_buffer[self.buffer_index + index % self.buffer_size];
    }
}

#[cfg(test)]
mod tests {
    use crate::node_v2::types::channel_pair;

    use super::*;

    #[test]
    fn cache_test() {
        let mut cache = Cache::<i64>::new(10);
        for i in 0..10 {
            cache.push(i, SocketId::new(), i);
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
        assert_eq!(cache.get_first(), &None);
    }

    #[test]
    fn node_thread_test() {
        // this test does not focus on predetermined implements.

        let (node_a_back, backend) = channel_pair::<BackendToNode, NodeToBackend>();
        let (node_a_front, frontend) = channel_pair::<FrontendToNode, NodeToFrontend>();

        let handle_node_a = std::thread::spawn(|| {});

        let (node_b_back, backend) = channel_pair::<BackendToNode, NodeToBackend>();
        let (node_b_front, frontend) = channel_pair::<FrontendToNode, NodeToFrontend>();

        let handle_node_b = std::thread::spawn(|| {});

        // wait for thread

        handle_node_a.join().unwrap();
        handle_node_b.join().unwrap();
    }
}
