use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::{Mutex, MutexGuard};

use crate::{
    err::NodeRemoveError,
    node::StopNode,
    plugin_manager::PluginManager,
    socket::{InputSocketCapsule, OutputSocketCapsule},
    FrameCount,
};

use super::{
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, UpdateInputDefaultError,
    },
    node::NodeCommon,
    types::{FromToBinary, NodeId, SharedAny, SocketId},
};

pub struct NodeController {
    // information as a node
    node_id: NodeId,
    node_name: Mutex<String>,

    // nodes
    nodes: HashMap<NodeId, Arc<dyn NodeCommon>>,

    // handle of node execution
    execution_handle: Option<tokio::task::JoinHandle<FrameCount>>,
    execution_stop_channel: Option<std::sync::mpsc::Sender<StopNode>>,
}

impl NodeController {
    pub fn new(name: &str) -> Self {
        NodeController {
            node_id: NodeId::new(),
            node_name: Mutex::new(name.to_string()),
            nodes: HashMap::new(),
            execution_handle: None,
            execution_stop_channel: None,
        }
    }

    // field operations

    pub fn add_node(&mut self, node: Arc<dyn NodeCommon>) {
        self.nodes.insert(node.get_id(), node);
    }

    pub async fn remove_node(&mut self, node_id: NodeId) -> Result<(), NodeRemoveError> {
        let e = self.node_disconnect_all(node_id).await;

        match e {
            Ok(_) | Err(NodeDisconnectError::NotConnected) => {
                self.nodes.remove(&node_id);

                Ok(())
            }
            Err(NodeDisconnectError::NodeIdNotFound) => {
                Err(NodeRemoveError::NodeIdNotFound)
            }
            Err(NodeDisconnectError::SocketIdNotFound) => Err(NodeRemoveError::DisconnectError(
                NodeDisconnectError::SocketIdNotFound,
            )),
        }
    }

    pub async fn check_consistency(&self) -> bool {
        for id in self.nodes.keys() {
            if *id != self.nodes.get(id).unwrap().get_id() {
                return false;
            }
        }
        true
    }

    pub async fn ensure_consistency(&mut self) {
        let mut ids = HashSet::new();
        for (id, node) in self.nodes.iter() {
            if *id != node.get_id() {
                ids.insert(*id);
            }
        }
        for id in ids {
            let node = self.nodes.remove(&id).unwrap();
            let id = node.get_id();
            self.nodes.insert(id, node);
        }
    }

    // node operations
    pub async fn node_connect(
        &self,
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeConnectError> {
        // get nodes
        let Some(upstream_node) = self.nodes.get(&upstream_node_id) else {
            return Err(NodeConnectError::NodeIdNotFound);
        };
        let Some(downstream_node) = self.nodes.get(&downstream_node_id) else {
            return Err(NodeConnectError::NodeIdNotFound);
        };

        // get sockets
        let Some(upstream_socket) = upstream_node
            .get_output_socket(upstream_node_socket_id)
            .await
        else {
            return Err(NodeConnectError::SocketIdNotFound);
        };
        let Some(downstream_socket) = downstream_node
            .get_input_socket(downstream_node_socket_id)
            .await
        else {
            return Err(NodeConnectError::SocketIdNotFound);
        };

        // connect
        crate::socket::connect(upstream_socket, downstream_socket).await
    }

    pub async fn node_conservative_connect(
        &self,
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeConnectError> {
        // get nodes
        let Some(upstream_node) = self.nodes.get(&upstream_node_id) else {
            return Err(NodeConnectError::NodeIdNotFound);
        };
        let Some(downstream_node) = self.nodes.get(&downstream_node_id) else {
            return Err(NodeConnectError::NodeIdNotFound);
        };

        // get sockets
        let Some(upstream_socket) = upstream_node
            .get_output_socket(upstream_node_socket_id)
            .await
        else {
            return Err(NodeConnectError::SocketIdNotFound);
        };
        let Some(downstream_socket) = downstream_node
            .get_input_socket(downstream_node_socket_id)
            .await
        else {
            return Err(NodeConnectError::SocketIdNotFound);
        };

        // connect
        crate::socket::conservative_connect(upstream_socket, downstream_socket).await
    }

    pub async fn node_disconnect(
        &self,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeDisconnectError> {
        // get nodes
        let Some(downstream_node) = self.nodes.get(&downstream_node_id) else {
            return Err(NodeDisconnectError::NodeIdNotFound);
        };

        // get sockets
        let Some(downstream_socket) = downstream_node
            .get_input_socket(downstream_node_socket_id)
            .await
        else {
            return Err(NodeDisconnectError::SocketIdNotFound);
        };

        // disconnect
        downstream_socket.disconnect().await
    }

    pub async fn node_conservative_disconnect(
        &self,
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeDisconnectError> {
        // get nodes
        let Some(upstream_node) = self.nodes.get(&upstream_node_id) else {
            return Err(NodeDisconnectError::NodeIdNotFound);
        };
        let Some(downstream_node) = self.nodes.get(&downstream_node_id) else {
            return Err(NodeDisconnectError::NodeIdNotFound);
        };

        // get sockets
        let Some(upstream_socket) = upstream_node
            .get_output_socket(upstream_node_socket_id)
            .await
        else {
            return Err(NodeDisconnectError::SocketIdNotFound);
        };
        let Some(downstream_socket) = downstream_node
            .get_input_socket(downstream_node_socket_id)
            .await
        else {
            return Err(NodeDisconnectError::SocketIdNotFound);
        };

        // check connection
        if upstream_socket
            .downstream_ids()
            .await
            .contains(&downstream_socket.socket_id())
        {
            // disconnect
            upstream_socket
                .disconnect(downstream_socket.socket_id())
                .await
        } else {
            Err(NodeDisconnectError::NotConnected)
        }
    }

    pub async fn check_connection(
        &self,
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeConnectionCheckError> {
        // get nodes
        let Some(upstream_node) = self.nodes.get(&upstream_node_id) else {
            return Err(NodeConnectionCheckError::NodeIdNotFound);
        };
        let Some(downstream_node) = self.nodes.get(&downstream_node_id) else {
            return Err(NodeConnectionCheckError::NodeIdNotFound);
        };

        // get sockets
        let Some(upstream_socket) = upstream_node
            .get_output_socket(upstream_node_socket_id)
            .await
        else {
            return Err(NodeConnectionCheckError::SocketIdNotFound);
        };
        let Some(downstream_socket) = downstream_node
            .get_input_socket(downstream_node_socket_id)
            .await
        else {
            return Err(NodeConnectionCheckError::SocketIdNotFound);
        };

        // check connection
        if upstream_socket
            .downstream_ids()
            .await
            .contains(&downstream_socket.socket_id())
        {
            Ok(())
        } else {
            Err(NodeConnectionCheckError::NotConnected)
        }
    }

    pub async fn node_disconnect_all_from_output(
        &mut self,
        node_id: NodeId,
        socket_id: SocketId,
    ) -> Result<(), NodeDisconnectError> {
        // get socket
        let Some(node) = self.nodes.get(&node_id) else {
            return Err(NodeDisconnectError::NodeIdNotFound);
        };
        let Some(socket) = node.get_output_socket(socket_id).await else {
            return Err(NodeDisconnectError::SocketIdNotFound);
        };

        // disconnect all
        socket.disconnect_all_output().await
    }

    /// Disconnect all input and output sockets of a node.
    pub async fn node_disconnect_all(
        &mut self,
        node_id: NodeId,
    ) -> Result<(), NodeDisconnectError> {
        // get node
        let Some(node) = self.nodes.get(&node_id) else {
            return Err(NodeDisconnectError::NodeIdNotFound);
        };

        // disconnect all output
        for socket in node.get_all_output_socket().await {
            socket.disconnect_all_output().await?;
        }

        // disconnect all input
        for socket in node.get_all_input_socket().await {
            socket.disconnect().await?;
        }

        Ok(())
    }

    // update input default of node
    pub async fn node_update_input_default(
        &self,
        node_id: NodeId,
        input_socket_id: SocketId,
        default: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError> {
        if let Some(node) = self.nodes.get(&node_id) {
            node.update_input_default(input_socket_id, default).await
        } else {
            Err(UpdateInputDefaultError::NodeIdNotFound(default))
        }
    }

    // main process

    pub async fn field_call(
        &self,
        frame: FrameCount,
        node_id: NodeId,
    ) -> Result<Arc<SharedAny>, ()> {
        if let Some(node) = self.nodes.get(&node_id) {
            Ok(node.call(frame).await)
        } else {
            Err(())
        }
    }

    // todo implement return value
    pub async fn field_play(&mut self, begin_frame: FrameCount, node_id: NodeId) -> Result<(), ()> {
        if let Some(node) = self.nodes.get(&node_id) {
            let (tx, rx) = std::sync::mpsc::channel();

            let node = node.clone();
            let handle = tokio::spawn(async move { node.play(begin_frame, rx).await });

            self.execution_handle = Some(handle);
            self.execution_stop_channel = Some(tx);

            Ok(())
        } else {
            Err(())
        }
    }

    pub fn field_is_playing(&self) -> bool {
        self.execution_handle.is_some()
    }

    pub async fn field_stop(&mut self) -> FrameCount {
        if let Some(tx) = self.execution_stop_channel.take() {
            tx.send(StopNode).unwrap();
        }

        let frame = self.execution_handle.take().unwrap().await.unwrap();
        self.execution_handle = None;

        frame
    }
}

#[async_trait::async_trait]
impl NodeCommon for NodeController {
    fn get_id(&self) -> NodeId {
        self.node_id
    }

    async fn get_name(&self) -> MutexGuard<'_, String> {
        self.node_name.lock().await
    }

    async fn set_name(&self, name: String) {
        *self.node_name.lock().await = name;
    }

    async fn change_cache_depth(&self, new_cache_size: usize) {
        todo!()
    }

    async fn clear_cache(&self) {
        todo!()
    }

    async fn cache_depth(&self) -> usize {
        todo!()
    }

    async fn cache_size(&self) -> usize {
        todo!()
    }

    async fn get_input_socket(&self, socket_id: SocketId) -> Option<InputSocketCapsule> {
        todo!()
    }

    async fn get_output_socket(&self, socket_id: SocketId) -> Option<OutputSocketCapsule> {
        todo!()
    }

    async fn get_all_output_socket(&self) -> Vec<OutputSocketCapsule> {
        todo!()
    }

    async fn get_all_input_socket(&self) -> Vec<InputSocketCapsule> {
        todo!()
    }

    async fn update_input_default(
        &self,
        input_socket_id: SocketId,
        default: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError> {
        todo!()
    }

    async fn call(&self, frame: FrameCount) -> Arc<SharedAny> {
        todo!()
    }

    async fn play(
        &self,
        frame: FrameCount,
        stop_channel: std::sync::mpsc::Receiver<StopNode>,
    ) -> FrameCount {
        todo!()
    }
}

/// binary operations
impl NodeController {
    pub async fn from_binary(
        plugin_manager: &PluginManager,
        binary: Arc<std::sync::RwLock<&[u8]>>,
    ) -> Result<Self, ()> {
        let seek: usize = 0;
        // read node field data
        todo!();

        // read nodes data
        todo!();

        // add nodes to node field
        todo!();
    }

    pub fn to_binary(&self) -> Vec<u8> {
        todo!()
    }
}

#[cfg(test)]
mod tests;
