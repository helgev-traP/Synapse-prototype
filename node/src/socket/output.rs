use std::{
    collections::{HashMap, HashSet}, ops::Deref, sync::{Arc, Weak}
};

use crate::{
    err::NodeDisconnectError,
    node_core::NodeCore,
    types::{SharedAny, SocketId},
    FrameCount,
};

use super::{InputGroup, InputTrait};

// inner data of Output

pub struct OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    // identifier
    id: SocketId,
    name: String,

    // call node's method to get data and pick up data
    pickup: Box<dyn Fn(&NodeProcessOutput) -> SocketType + Send + Sync>,
    // main body of node
    node: Arc<NodeCore<NodeInputs, NodeMemory, NodeProcessOutput>>,

    // downstream sockets
    downstream: tokio::sync::Mutex<HashMap<SocketId, Weak<dyn InputTrait>>>,
}

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
    ) -> Self {
        OutputSocket {
            id: SocketId::new(),
            name: name.to_string(),
            pickup,
            node: main_body_of_node,
            downstream: tokio::sync::Mutex::new(HashMap::new()),
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
    fn get_id(&self) -> SocketId {
        self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn get_downstream_ids(&self) -> HashSet<SocketId> {
        self.downstream.lock().await.keys().copied().collect()
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<SocketType>()
    }

    async fn connect(&self, socket: Weak<dyn InputTrait>) {
        let id = socket.upgrade().unwrap().get_id();
        self.downstream.lock().await.insert(id, socket);
    }

    async fn disconnect(&self, downstream_id: SocketId) -> Result<(), NodeDisconnectError> {
        if self.downstream.lock().await.remove(&downstream_id).is_none() {
            return Err(NodeDisconnectError::NotConnected);
        }
        Ok(())
    }

    async fn call(&self, frame: FrameCount) -> Box<SharedAny> {
        let output = self.node.call(frame).await;

        Box::new((self.pickup)(&output)) as Box<SharedAny>
    }

    async fn clear_cache(&self) {
        // let all downstream nodes clear cache
        for (_, socket) in self.downstream.lock().await.iter() {
            if let Some(socket) = socket.upgrade() {
                socket.clear_cache().await;
            }
        }
    }
}

#[async_trait::async_trait]
pub(crate) trait OutputTrait: Send + Sync {
    // --- use from NodeField ---
    // getters
    fn get_id(&self) -> SocketId;
    fn get_name(&self) -> &str;

    // get downstream ids
    async fn get_downstream_ids(&self) -> HashSet<SocketId>;

    // connect and disconnect
    fn type_id(&self) -> std::any::TypeId;
    async fn connect(&self, socket: Weak<dyn InputTrait>);
    async fn disconnect(&self, downstream_id: SocketId) -> Result<(), NodeDisconnectError>;

    // --- use from InputSocket ---
    // called by downstream socket
    async fn call(&self, frame: FrameCount) -> Box<SharedAny>;

    // called by upstream socket
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
    pub fn new(s: Arc<dyn OutputTrait>) -> Self {
        OutputTree::Socket(s)
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
    pub fn get_socket(&self, id: SocketId) -> Option<Weak<dyn OutputTrait>> {
        match self {
            OutputTree::Socket(s) => {
                if s.get_id() == id {
                    Some(Arc::downgrade(s))
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

    pub async fn clear_cache(&self) {
        match self {
            OutputTree::Socket(s) => {
                let _ = s.clear_cache().await;
            }
            OutputTree::Vec(v) => {
                for x in v {
                    x.clear_cache();
                }
            }
        }
    }
}
