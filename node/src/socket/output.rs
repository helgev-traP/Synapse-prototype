use core::panic;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::{
    channel::{NodeOrder, NodeResponse, OutputChannel},
    err::{NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, NodeSendResponseError},
    node_core::NodeCore,
    types::{NodeId, SharedAny, SocketId, TryRecvResult}, FrameCount,
};

use super::{InputCommon, InputTree};

// inner data of Output

struct OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputTree + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
    NodeOutputs: OutputTree + Send + 'static,
{
    // identifier
    id: SocketId,
    name: String,

    // call node's method to get data and pick up data
    pickup: Box<dyn Fn(&NodeProcessOutput) -> SocketType + Send + Sync>,
    // main body of node
    node: Arc<NodeCore<NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>>,

    // downstream sockets
    downstream: HashMap<SocketId, Weak<dyn InputCommon>>,
}

impl<SocketType, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
    OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputTree + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
    NodeOutputs: OutputTree + Send + 'static,
{
    pub fn new(
        name: String,
        pickup: Box<dyn Fn(&NodeProcessOutput) -> SocketType + Send + Sync>,
        main_body_of_node: Arc<NodeCore<NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>>,
    ) -> Self {
        OutputSocket {
            id: SocketId::new(),
            name,
            pickup,
            node: main_body_of_node,
            downstream: HashMap::new(),
        }
    }

    pub async fn send_delete_cache(&mut self) {
        todo!()
    }
}

#[async_trait::async_trait]
impl<SocketType, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs> OutputCommon for OutputSocket<SocketType, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
where
    SocketType: Clone + Send + Sync + 'static,
    NodeInputs: InputTree + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
    NodeOutputs: OutputTree + Send + 'static,
{
    fn get_id(&self) -> SocketId {
        self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_downstream_ids(&self) -> HashSet<SocketId> {
        self.downstream.keys().copied().collect()
    }

    async fn connect(&self, socket: Weak<dyn InputCommon>) -> Result<(), NodeConnectError> {
        todo!()
    }

    async fn disconnect(&self, id: &SocketId) -> Result<(), NodeDisconnectError> {
        todo!()
    }

    async fn call(&self, frame: FrameCount) -> SharedAny {
        todo!()
    }
}

#[async_trait::async_trait]
pub(crate) trait OutputCommon: Send + Sync + 'static {
    // --- use from NodeField ---
    // getters
    fn get_id(&self) -> SocketId;
    fn get_name(&self) -> &str;

    // get downstream ids
    fn get_downstream_ids(&self) -> HashSet<SocketId>;

    // connect and disconnect
    async fn connect(&self, socket: Weak<dyn InputCommon>) -> Result<(), NodeConnectError>;
    async fn disconnect(&self, id: &SocketId) -> Result<(), NodeDisconnectError>;

    // --- use from InputSocket ---
    // called by downstream socket
    async fn call(&self, frame: FrameCount) -> SharedAny;
}

#[async_trait::async_trait]
pub trait OutputTree: Send + 'static {
    async fn get_socket(&self, id: &SocketId) -> Option<Weak<dyn OutputCommon>>;
    async fn clear_cache(&self);
}
