use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};

use crate::{
    err::NodeDisconnectError,
    node_core::NodeCore,
    types::{SharedAny, SocketId},
    FrameCount,
};

use super::{InputGroup, InputSocketCapsule};

pub struct OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    // Weak to self
    weak: Weak<OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>>,

    // identifier
    id: SocketId,
    name: String,

    // call node's method to get data and pick up data
    pickup: Box<dyn Fn(&NodeProcessOutput) -> SocketType + Send + Sync>,
    // main body of node
    node: Arc<NodeCore<NodeInputs, NodeMemory, NodeProcessOutput>>,

    // downstream sockets
    downstream: tokio::sync::Mutex<HashMap<SocketId, InputSocketCapsule>>,
}

/// construct OutputSocket
impl<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>
    OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    pub fn new(
        name: &str,
        pickup: Box<dyn Fn(&NodeProcessOutput) -> SocketType + Send + Sync>,
        main_body_of_node: Arc<NodeCore<NodeInputs, NodeMemory, NodeProcessOutput>>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak| OutputSocket {
            weak: weak.clone(),
            id: SocketId::new(),
            name: name.to_string(),
            pickup,
            node: main_body_of_node,
            downstream: tokio::sync::Mutex::new(HashMap::new()),
        })
    }
}

impl<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>
    OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    fn make_capsule(&self) -> OutputSocketCapsule {
        OutputSocketCapsule {
            socket_id: self.id,
            socket_type: std::any::TypeId::of::<SocketType>(),
            weak: self.weak.clone() as Weak<dyn OutputTrait>,
        }
    }
}

#[async_trait::async_trait]
impl<SocketType, NodeInputs, NodeMemory, NodeProcessOutput> OutputTrait
    for OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    fn socket_id(&self) -> SocketId {
        self.id
    }

    fn socket_name(&self) -> &str {
        &self.name
    }

    async fn downstream_ids(&self) -> HashSet<SocketId> {
        self.downstream.lock().await.keys().copied().collect()
    }

    fn weak(&self) -> Weak<dyn OutputTrait> {
        self.weak.clone() as Weak<dyn OutputTrait>
    }

    fn socket_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<SocketType>()
    }

    async fn connect(&self, socket: InputSocketCapsule) {
        let id = socket.socket_id();
        self.downstream.lock().await.insert(id, socket);
    }

    async fn disconnect(&self, downstream_id: SocketId) -> Result<(), NodeDisconnectError> {
        if self
            .downstream
            .lock()
            .await
            .remove(&downstream_id)
            .is_none()
        {
            return Err(NodeDisconnectError::NotConnected);
        }
        Ok(())
    }

    async fn disconnect_all_output(&self) -> Result<(), NodeDisconnectError> {
        // disconnect all downstream nodes
        for socket in self.downstream.lock().await.values() {
            socket.disconnect_called_from_upstream().await.unwrap();
        }

        // clear downstream nodes
        self.downstream.lock().await.clear();

        Ok(())
    }

    async fn call(&self, frame: FrameCount) -> Box<SharedAny> {
        let output = self.node.call(frame).await;

        Box::new((self.pickup)(&output)) as Box<SharedAny>
    }

    async fn clear_cache(&self) {
        // let all downstream nodes clear cache
        for (_, socket) in self.downstream.lock().await.iter() {
            socket.clear_cache();
        }
    }
}

#[async_trait::async_trait]
pub(crate) trait OutputTrait: Send + Sync {
    // --- use from NodeField ---
    // getters
    fn socket_id(&self) -> SocketId;
    fn socket_name(&self) -> &str;

    // get downstream ids
    async fn downstream_ids(&self) -> HashSet<SocketId>;

    // connect and disconnect
    fn weak(&self) -> Weak<dyn OutputTrait>;
    fn socket_type(&self) -> std::any::TypeId;
    async fn connect(&self, socket: InputSocketCapsule);
    async fn disconnect(&self, downstream_id: SocketId) -> Result<(), NodeDisconnectError>;
    async fn disconnect_all_output(&self) -> Result<(), NodeDisconnectError>;

    // --- use from InputSocket ---
    // called by downstream socket
    async fn call(&self, frame: FrameCount) -> Box<SharedAny>;

    // let all downstream nodes clear cache
    async fn clear_cache(&self);
}

pub enum OutputTree {
    Socket(Arc<dyn OutputTrait>),
    Vec(Vec<OutputTree>),
}

// indexing
impl std::ops::Index<usize> for OutputTree {
    type Output = OutputTree;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            OutputTree::Vec(v) => &v[index],
            _ => panic!("OutputTree::Socket cannot be indexed"),
        }
    }
}

impl OutputTree {
    pub fn new<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>(
        socket: Arc<OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>>,
    ) -> Self
    where
        SocketType: Clone + Send + Sync + 'static,
        NodeInputs: InputGroup + Send + Sync + 'static,
        NodeMemory: Send + Sync + 'static,
        NodeProcessOutput: Clone + Send + Sync + 'static,
    {
        OutputTree::Socket(socket as Arc<dyn OutputTrait>)
    }

    pub fn vec(v: Vec<OutputTree>) -> Self {
        OutputTree::Vec(v)
    }

    pub fn empty() -> Self {
        OutputTree::Vec(Vec::new())
    }

    pub fn push(&mut self, s: OutputTree) {
        match self {
            OutputTree::Vec(v) => v.push(s),
            OutputTree::Socket(_) => {
                let mut swap = OutputTree::empty();
                std::mem::swap(self, &mut swap);
                self.push(swap);
                self.push(s);
            }
        }
    }

    pub fn insert(&mut self, index: usize, s: OutputTree) {
        match self {
            OutputTree::Vec(v) => v.insert(index, s),
            _ => panic!("OutputTree::Socket cannot be indexed"),
        }
    }

    pub fn remove(&mut self, index: usize) -> OutputTree {
        match self {
            OutputTree::Vec(v) => v.remove(index),
            _ => panic!("OutputTree::Socket cannot be indexed"),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            OutputTree::Vec(v) => v.iter().map(|x| x.size()).sum(),
            _ => 1,
        }
    }
}

impl OutputTree {
    pub fn get_socket(&self, id: SocketId) -> Option<OutputSocketCapsule> {
        match self {
            OutputTree::Socket(s) => {
                if s.socket_id() == id {
                    Some(OutputSocketCapsule::from_output_trait(s.as_ref()))
                } else {
                    None
                }
            }
            OutputTree::Vec(v) => {
                for x in v {
                    if let Some(s) = x.get_socket(id) {
                        return Some(s);
                    }
                }
                None
            }
        }
    }

    pub fn get_all_socket(&self) -> Vec<OutputSocketCapsule> {
        match self {
            OutputTree::Socket(s) => vec![OutputSocketCapsule::from_output_trait(s.as_ref())],
            OutputTree::Vec(v) => v.iter().flat_map(|x| x.get_all_socket()).collect(),
        }
    }

    pub fn clear_cache(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            match self {
                OutputTree::Socket(s) => {
                    let _ = s.clear_cache().await;
                }
                OutputTree::Vec(v) => {
                    for x in v {
                        x.clear_cache().await;
                    }
                }
            }
        })
    }
}

// WeakOutputSocket
// to transfer OutputSocket to another thread
pub struct OutputSocketCapsule {
    socket_id: SocketId,
    socket_type: std::any::TypeId,
    weak: Weak<dyn OutputTrait>,
}

impl Clone for OutputSocketCapsule {
    fn clone(&self) -> Self {
        OutputSocketCapsule {
            socket_id: self.socket_id,
            socket_type: self.socket_type,
            weak: self.weak.clone(),
        }
    }
}

impl OutputSocketCapsule {
    fn from_output_trait(output: &dyn OutputTrait) -> Self {
        OutputSocketCapsule {
            socket_id: output.socket_id(),
            socket_type: output.socket_type(),
            weak: output.weak(),
        }
    }
}

impl OutputSocketCapsule {
    pub fn socket_id(&self) -> SocketId {
        self.socket_id
    }

    pub fn socket_type(&self) -> std::any::TypeId {
        self.socket_type
    }

    pub async fn downstream_ids(&self) -> HashSet<SocketId> {
        // todo: add error handling when weak reference is invalid

        if let Some(output) = self.weak.upgrade() {
            output.downstream_ids().await
        } else {
            todo!()
        }
    }

    pub async fn connect(&self, socket: InputSocketCapsule) {
        // todo: add error handling when weak reference is invalid

        if let Some(output) = self.weak.upgrade() {
            output.connect(socket).await;
        }
    }

    pub async fn disconnect(&self, downstream_id: SocketId) -> Result<(), NodeDisconnectError> {
        // todo: add error handling when weak reference is invalid

        if let Some(output) = self.weak.upgrade() {
            output.disconnect(downstream_id).await
        } else {
            todo!()
        }
    }

    pub async fn disconnect_all_output(&self) -> Result<(), NodeDisconnectError> {
        // todo: add error handling when weak reference is invalid

        if let Some(output) = self.weak.upgrade() {
            output.disconnect_all_output().await
        } else {
            todo!()
        }
    }

    pub async fn call(&self, frame: FrameCount) -> Box<SharedAny> {
        // todo: add error handling when weak reference is invalid

        if let Some(output) = self.weak.upgrade() {
            output.call(frame).await
        } else {
            todo!()
        }
    }

    pub async fn clear_cache(&self) {
        // todo: add error handling when weak reference is invalid

        if let Some(output) = self.weak.upgrade() {
            output.clear_cache().await;
        }
    }
}
