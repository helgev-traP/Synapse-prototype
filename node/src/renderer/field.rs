use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::mpsc;
use uuid::Uuid;

use super::{
    channel::{
        channel_pair, FieldChannel, FrontToField, FrontToFieldResult, InputChannel, OutputChannel,
    },
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, UpdateInputDefaultError,
    },
    node::NodeCoreCommon,
    types::{FromToBinary, NodeId, SharedAny, SocketId},
};

// todo いらないかも
pub trait NodeFieldCommon {
    fn add_node(&mut self, node: Box<dyn NodeCoreCommon>);
    fn remove_node(&mut self, node_id: &NodeId);
    fn get_node(
        &self,
        node_id: &NodeId,
    ) -> Option<Arc<std::sync::Mutex<Box<dyn NodeCoreCommon + 'static>>>>;
    fn check_consistency(&self) -> bool;
    fn ensure_consistency(&mut self);
    // main loop
    fn main_loop();
}

pub struct NodeField {
    node_id: NodeId,
    node_name: String,
    nodes: HashMap<NodeId, Arc<tokio::sync::Mutex<Box<dyn NodeCoreCommon>>>>,
    channel_front: tokio::sync::Mutex<FieldChannel>,
}

impl NodeField {
    pub fn new(name: String, channel: FieldChannel) -> Self {
        NodeField {
            node_id: NodeId::new(),
            node_name: name,
            nodes: HashMap::new(),
            channel_front: tokio::sync::Mutex::new(channel),
        }
    }

    // field operations

    pub fn add_node(&mut self, node: Box<dyn NodeCoreCommon>) {
        self.nodes.insert(
            node.get_id().clone(),
            Arc::new(tokio::sync::Mutex::new(node)),
        );
    }

    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.nodes.remove(node_id);
    }

    // fn get_node(&self, node_id: NodeId) -> Option<&Box<dyn NodeCoreCommon<'static>>> {
    //     match self.nodes.get(&node_id).as_ref() {
    //         Some(node) => Some(node.lock().await.as_ref()),
    //         None => None,
    //     }
    // }

    pub async fn check_consistency(&self) -> bool {
        for id in self.nodes.keys() {
            if id != self.nodes.get(id).unwrap().lock().await.get_id() {
                return false;
            }
        }
        return true;
    }

    pub async fn ensure_consistency(&mut self) {
        let mut ids = HashSet::new();
        for (id, node) in self.nodes.iter() {
            if id != node.lock().await.get_id() {
                ids.insert(id.clone());
            }
        }
        for id in ids {
            let node = self.nodes.remove(&id).unwrap();
            let id = node.lock().await.get_id().clone();
            self.nodes.insert(id, node);
        }
    }

    // node operations
    pub async fn node_force_connect(
        &self,
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeConnectError> {
        if let Some(upstream_node) = self.nodes.get(&upstream_node_id) {
            if let Some(downstream_node) = self.nodes.get(&downstream_node_id) {
                // prepare channel
                let (ch_upstream, ch_downstream): (OutputChannel, InputChannel) = channel_pair(1);

                // clone for task
                let upstream_node = upstream_node.clone();
                let downstream_node = downstream_node.clone();

                // spawn task for each node
                let upstream_handle = tokio::spawn(async move {
                    let upstream_node = upstream_node.lock().await;
                    upstream_node
                        .connect_output(
                            ch_upstream,
                            &upstream_node_socket_id,
                            &downstream_node_socket_id,
                        )
                        .await
                });
                let downstream_handle = tokio::spawn(async move {
                    let mut downstream_node = downstream_node.lock().await;
                    downstream_node
                        .connect_input(ch_downstream, &downstream_node_socket_id)
                        .await
                });

                // wait
                upstream_handle.await.unwrap()?;
                downstream_handle.await.unwrap()?;
                return Ok(());
            } else {
                return Err(NodeConnectError::NodeIdNotFound);
            }
        } else {
            return Err(NodeConnectError::NodeIdNotFound);
        }
    }

    pub async fn node_disconnect(
        &self,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeDisconnectError> {
        if let Some(downstream_node) = self.nodes.get(&downstream_node_id) {
            downstream_node
                .lock()
                .await
                .disconnect_input(&downstream_node_socket_id)
                .await?;
            Ok(())
        } else {
            Err(NodeDisconnectError::NodeIdNotFound)
        }
    }

    pub async fn node_disconnect_safe(
        &self,
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeDisconnectError> {
        if let Some(upstream_node) = self.nodes.get(&upstream_node_id) {
            if let Some(downstream_node) = self.nodes.get(&downstream_node_id) {
                // check connection
                match Self::check_connection(
                    upstream_node,
                    upstream_node_socket_id,
                    downstream_node,
                    downstream_node_socket_id,
                )
                .await
                {
                    Ok(_) => {
                        // disconnect
                        upstream_node
                            .lock()
                            .await
                            .disconnect_output(&upstream_node_socket_id, &downstream_node_socket_id)
                            .await
                            .unwrap();
                        downstream_node
                            .lock()
                            .await
                            .disconnect_input(&downstream_node_socket_id)
                            .await
                            .unwrap();
                        return Ok(());
                    }
                    Err(err) => match err {
                        NodeConnectionCheckError::NodeIdNotFound => {
                            Err(NodeDisconnectError::NodeIdNotFound)
                        }
                        NodeConnectionCheckError::SocketIdNotFound => {
                            Err(NodeDisconnectError::SocketIdNotFound)
                        }
                        NodeConnectionCheckError::ChannelClosed
                        | NodeConnectionCheckError::NotConnected => {
                            Err(NodeDisconnectError::NotConnected)
                        }
                    },
                }
            } else {
                return Err(NodeDisconnectError::NodeIdNotFound);
            }
        } else {
            return Err(NodeDisconnectError::NodeIdNotFound);
        }
    }

    pub async fn check_connection(
        upstream_node: &Arc<tokio::sync::Mutex<Box<dyn NodeCoreCommon>>>,
        upstream_node_socket_id: SocketId,
        downstream_node: &Arc<tokio::sync::Mutex<Box<dyn NodeCoreCommon>>>,
        downstream_node_socket_id: SocketId,
    ) -> Result<(), NodeConnectionCheckError> {
        let token = Uuid::new_v4();
        upstream_node
            .lock()
            .await
            .send_connection_checker(
                token.clone(),
                &upstream_node_socket_id,
                &downstream_node_socket_id,
            )
            .await?;
        let received_token = downstream_node
            .lock()
            .await
            .recv_connection_checker(&downstream_node_socket_id)
            .await?;
        if token == received_token {
            return Ok(());
        } else {
            return Err(NodeConnectionCheckError::NotConnected);
        }
    }

    pub async fn node_disconnect_from_input_socket(
        &mut self,
        node_id: &NodeId,
        socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        // get output socket id and node id
        todo!()
    }

    pub async fn node_disconnect_from_output_socket(
        &mut self,
        node_id: &NodeId,
        socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        // get input socket id and node id
        todo!()
    }

    // update input default of node
    pub async fn update_input_default(
        &self,
        node_id: &NodeId,
        input_socket_id: &SocketId,
        default: SharedAny,
    ) -> Result<(), UpdateInputDefaultError> {
        if let Some(node) = self.nodes.get(node_id) {
            node.lock()
                .await
                .update_input_default(input_socket_id, default)
                .await
        } else {
            Err(UpdateInputDefaultError::NodeIdNotFound(default))
        }
    }

    // main loop

    pub async fn main_loop(&self, millis: u64) {
        // tokio 時間計測
        let timeout = tokio::time::Duration::from_millis(millis);

        // set runtime information
        let timer = tokio::time::Instant::now();
        let id_set = Arc::new(tokio::sync::Mutex::new(HashSet::<NodeId>::new()));
        'main_loop: loop {
            for (id, node) in self.nodes.iter() {
                // check message
                if timer.elapsed() >= timeout {
                    // receive message from frontend
                    'cache_message: loop {
                        let id_set_clone = id_set.clone();
                        match self
                            .channel_front
                            .lock()
                            .await
                            .try_recv_generic(Box::new(|message: FrontToField| async {
                                let mut if_shutdown = false;
                                let result = match message {
                                    FrontToField::Shutdown => {
                                        loop {
                                            if id_set_clone.lock().await.is_empty() {
                                                break;
                                            }
                                        }
                                        if_shutdown = true;
                                        FrontToFieldResult::Shutdown(Ok(()))
                                    }
                                    FrontToField::NodeConnect {
                                        upstream_node_id,
                                        upstream_node_socket_id,
                                        downstream_node_id,
                                        downstream_node_socket_id,
                                    } => {
                                        match self
                                            .node_force_connect(
                                                upstream_node_id,
                                                upstream_node_socket_id,
                                                downstream_node_id,
                                                downstream_node_socket_id,
                                            )
                                            .await
                                        {
                                            Ok(_) => FrontToFieldResult::NodeConnect(Ok(())),
                                            Err(err) => FrontToFieldResult::NodeConnect(Err(err)),
                                        }
                                    }
                                    FrontToField::NodeDisconnect {
                                        downstream_node_id,
                                        downstream_node_socket_id,
                                    } => {
                                        match self
                                            .node_disconnect(
                                                downstream_node_id,
                                                downstream_node_socket_id,
                                            )
                                            .await
                                        {
                                            Ok(_) => FrontToFieldResult::NodeDisconnect(Ok(())),
                                            Err(err) => {
                                                FrontToFieldResult::NodeDisconnect(Err(err))
                                            }
                                        }
                                    }
                                    FrontToField::NodeDisconnectSafe {
                                        upstream_node_id,
                                        upstream_node_socket_id,
                                        downstream_node_id,
                                        downstream_node_socket_id,
                                    } => {
                                        match self
                                            .node_disconnect_safe(
                                                upstream_node_id,
                                                upstream_node_socket_id,
                                                downstream_node_id,
                                                downstream_node_socket_id,
                                            )
                                            .await
                                        {
                                            Ok(_) => FrontToFieldResult::NodeDisconnectSafe(Ok(())),
                                            Err(err) => {
                                                FrontToFieldResult::NodeDisconnectSafe(Err(err))
                                            }
                                        }
                                    }
                                    FrontToField::UpdateInputDefaultValue {
                                        node_id,
                                        socket_id,
                                        value,
                                    } => FrontToFieldResult::UpdateInputDefaultValue(
                                        self.update_input_default(&node_id, &socket_id, value)
                                            .await,
                                    ),
                                };
                                (result, if_shutdown)
                            }))
                            .await
                        {
                            Ok(if_shutdown) => {
                                if if_shutdown {
                                    break 'main_loop;
                                }
                            }
                            Err(err) => match err {
                                mpsc::error::TryRecvError::Empty => {
                                    break 'cache_message;
                                }
                                mpsc::error::TryRecvError::Disconnected => todo!(),
                            },
                        }
                    }
                }
                // check if node is already running
                if id_set.lock().await.contains(id) {
                    continue;
                }

                // insert node id to id_set
                id_set.lock().await.insert(id.clone());
                // execute node's main
                let node = node.clone();
                let id_set = id_set.clone();

                // task spawn
                tokio::spawn(async move {
                    let mut node = node.lock().await;
                    (*node).main().await;
                    id_set.lock().await.remove(node.get_id());
                });
            }
        }
    }
}

#[async_trait::async_trait]
impl NodeCoreCommon for NodeField {
    fn get_id(&self) -> &NodeId {
        todo!()
    }

    fn get_name(&self) -> &String {
        todo!()
    }

    fn set_name(&mut self, name: String) {
        todo!()
    }
    async fn change_cache_depth(&mut self, new_cache_size: usize) {
        todo!()
    }

    fn clear_cache(&mut self) {
        todo!()
    }

    async fn cache_depth(&self) -> usize {
        todo!()
    }

    fn cache_size(&self) -> usize {
        todo!()
    }

    async fn connect_output(
        &self,
        channel: OutputChannel,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        todo!()
    }

    async fn disconnect_output(
        &self,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        todo!()
    }

    async fn send_connection_checker(
        &self,
        token: Uuid,
        socket_id: &SocketId,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectionCheckError> {
        todo!()
    }

    async fn connect_input(
        &mut self,
        channel: InputChannel,
        socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        todo!()
    }

    async fn disconnect_input(&mut self, socket_id: &SocketId) -> Result<(), NodeDisconnectError> {
        todo!()
    }

    async fn recv_connection_checker(
        &self,
        socket_id: &SocketId,
    ) -> Result<Uuid, NodeConnectionCheckError> {
        todo!()
    }

    async fn update_input_default(
        &mut self,
        input_socket_id: &SocketId,
        default: SharedAny,
    ) -> Result<(), UpdateInputDefaultError> {
        todo!()
    }

    async fn main(&mut self) {
        todo!()
    }
}

#[async_trait::async_trait]
impl FromToBinary for NodeField {
    async fn from_binary(binary: Arc<std::sync::RwLock<&[u8]>>) -> Result<Self, ()> {
        let seek: usize = 0;
        // read node field data
        todo!();

        // read nodes data
        todo!();

        // add nodes to node field
        todo!();
    }

    fn to_binary(&self) -> Vec<u8> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::any::TypeId;

    use crate::renderer::{
        channel::{result_channel_pair, FrontToFieldResult, NodeOrder, NodeResponse},
        node::NodeCore,
        socket::{Input, InputCommon, InputTree, Output, OutputTree},
        types::{Envelope, NodeName, SharedAny},
        FrameCount,
    };

    use super::*;

    use nodes::*;

    #[tokio::test]
    async fn hash_map_ensure_consistency_test() {
        // todo
    }

    async fn connect_and_disconnect_test_onetime(millis: u64) {
        // field
        let (mut field_operation, field) = result_channel_pair(1);
        let mut field = NodeField::new("field".to_string(), field);

        // node i64, string
        let (_, mut node_u64, node_u64_output, _) = get_u64_node("node_a".to_string(), 0);
        let (node_string_input, mut node_string, node_string_output) =
            get_string_node("node_b".to_string(), "node_b.".to_string());

        // multiple node
        let (u64_input, string_input, mut node_multiple, node_multiple_output) =
            get_multiple_node("node_multiple".to_string(), 1, "node_multiple.".to_string());

        // below is todo

        // channel to connect multiple node and operator.
        let (mut ch_call, dis_ch_out) = channel_pair(1);

        // ids for operation
        let node_u64_id = node_u64.get_id().clone();
        let node_string_id = node_string.get_id().clone();
        let node_multiple_id = node_multiple.get_id().clone();

        // set cache depth
        node_u64.change_cache_depth(10).await;
        node_string.change_cache_depth(10).await;
        node_multiple.change_cache_depth(10).await;

        // check cache depth
        assert_eq!(node_u64.cache_depth().await, 10);
        assert_eq!(node_string.cache_depth().await, 10);
        assert_eq!(node_multiple.cache_depth().await, 10);

        // check cache size
        assert_eq!(node_u64.cache_size(), 20);
        assert_eq!(node_string.cache_size(), 10);
        assert_eq!(node_multiple.cache_size(), 10);

        // node field
        let handle_field = tokio::spawn(async move {
            // connect display node to display operator
            node_multiple
                .connect_output(dis_ch_out, &node_multiple_output, &SocketId::new())
                .await
                .unwrap();

            // add nodes
            field.add_node(node_u64);
            field.add_node(node_string);
            field.add_node(node_multiple);

            field.main_loop(millis).await;
        });

        // operation to field. this works as a virtual frontend and command to display node.
        let handle_operator = tokio::spawn(async move {
            // connect display node to display operator
            let recv_result = ch_call.recv().await;
            if let Some(response) = recv_result {
                if let NodeResponse::CompatibleCheck { type_id, .. } = response {
                    assert_eq!(type_id, TypeId::of::<String>());
                    ch_call.send(NodeOrder::TypeConformed).await.unwrap();
                } else {
                    panic!();
                }
            } else {
                panic!();
            }

            // 1: "node_multiple."
            ch_call.send(NodeOrder::Request { frame: 0 }).await.unwrap();

            match ch_call.recv().await.unwrap() {
                NodeResponse::Shared(shared) => {
                    assert_eq!(shared.downcast_ref::<String>().unwrap(), "node_multiple.");
                }
                _ => panic!(),
            }

            // connect node_string -> "node_a"
            match field_operation
                .send(FrontToField::NodeConnect {
                    upstream_node_id: node_u64_id.clone(),
                    upstream_node_socket_id: node_u64_output.clone(),
                    downstream_node_id: node_multiple_id.clone(),
                    downstream_node_socket_id: u64_input.clone(),
                })
                .await
                .unwrap()
            {
                Some(result) => match result {
                    FrontToFieldResult::NodeConnect(result) => {
                        assert_eq!(result, Ok(()));
                    }
                    _ => panic!(),
                },
                None => todo!(),
            }

            if let NodeResponse::DeleteCache = ch_call.recv().await.unwrap() {
            } else {
                panic!();
            }

            ch_call.send(NodeOrder::Request { frame: 2 }).await.unwrap();

            match ch_call.recv().await.unwrap() {
                NodeResponse::Shared(shared) => {
                    assert_eq!(
                        shared.downcast_ref::<String>().unwrap(),
                        "node_multiple.node_multiple."
                    );
                }
                _ => panic!(),
            }

            // connect node_u64 to string input of node_multiple. (will failed because of type mismatch and nothing will be changed.)
            match field_operation
                .send(FrontToField::NodeConnect {
                    upstream_node_id: node_u64_id,
                    upstream_node_socket_id: node_u64_output,
                    downstream_node_id: node_multiple_id,
                    downstream_node_socket_id: string_input,
                })
                .await
                .unwrap()
            {
                Some(result) => match result {
                    FrontToFieldResult::NodeConnect(result) => {
                        assert_eq!(result, Err(NodeConnectError::TypeRejected));
                    }
                    _ => panic!(),
                },
                None => panic!(),
            }

            ch_call.send(NodeOrder::Request { frame: 2 }).await.unwrap();

            match ch_call.recv().await.unwrap() {
                NodeResponse::Shared(shared) => {
                    assert_eq!(
                        shared.downcast_ref::<String>().unwrap(),
                        "node_multiple.node_multiple."
                    );
                }
                _ => panic!(),
            }

            // connect node_string to string input of node_multiple. (will success.)
            match field_operation
                .send(FrontToField::NodeConnect {
                    upstream_node_id: node_string_id.clone(),
                    upstream_node_socket_id: node_string_output.clone(),
                    downstream_node_id: node_multiple_id.clone(),
                    downstream_node_socket_id: string_input.clone(),
                })
                .await
                .unwrap()
            {
                Some(result) => match result {
                    FrontToFieldResult::NodeConnect(result) => {
                        assert_eq!(result, Ok(()));
                    }
                    _ => panic!(),
                },
                None => panic!(),
            }

            if let NodeResponse::DeleteCache = ch_call.recv().await.unwrap() {
            } else {
                panic!();
            }

            ch_call.send(NodeOrder::Request { frame: 2 }).await.unwrap();

            match ch_call.recv().await.unwrap() {
                NodeResponse::Shared(shared) => {
                    assert_eq!(shared.downcast_ref::<String>().unwrap(), "node_b.node_b.");
                }
                _ => panic!(),
            };

            // dummy update
            match field_operation
                .send(FrontToField::UpdateInputDefaultValue {
                    node_id: node_string_id.clone(),
                    socket_id: node_string_input.clone(),
                    value: Box::new("dummy.".to_string()),
                })
                .await
                .unwrap()
            {
                Some(result) => match result {
                    FrontToFieldResult::UpdateInputDefaultValue(result) => {
                        if let Err(err) = result {
                            panic!("{:?}", err);
                        }
                    }
                    _ => panic!(),
                },
                None => panic!(),
            }

            // change node_b's default value to "node_b_new"
            match field_operation
                .send(FrontToField::UpdateInputDefaultValue {
                    node_id: node_string_id.clone(),
                    socket_id: node_string_input.clone(),
                    value: Box::new("node_b_new.".to_string()),
                })
                .await
                .unwrap()
            {
                Some(result) => match result {
                    FrontToFieldResult::UpdateInputDefaultValue(result) => {
                        if let Err(err) = result {
                            panic!("{:?}", err);
                        }
                    }
                    _ => panic!(),
                },
                None => panic!(),
            }

            if let NodeResponse::DeleteCache = ch_call.recv().await.unwrap() {
            } else {
                panic!();
            }

            if let NodeResponse::DeleteCache = ch_call.recv().await.unwrap() {
            } else {
                panic!();
            }

            ch_call.send(NodeOrder::Request { frame: 2 }).await.unwrap();

            match ch_call.recv().await.unwrap() {
                NodeResponse::Shared(shared) => {
                    assert_eq!(
                        shared.downcast_ref::<String>().unwrap(),
                        "node_b_new.node_b_new."
                    );
                }
                _ => panic!(),
            };

            // shutdown
            match field_operation.send(FrontToField::Shutdown).await.unwrap() {
                Some(result) => match result {
                    FrontToFieldResult::Shutdown(result) => {
                        assert_eq!(result, Ok(()));
                    }
                    _ => panic!(),
                },
                None => panic!(),
            }
        });

        handle_field.await.unwrap();
        handle_operator.await.unwrap();
    }

    #[tokio::test]
    async fn force_connect_and_disconnect_test() {
        for i in 0..10 {
            for _ in 0..10 {
                connect_and_disconnect_test_onetime(i).await;
            }
        }
    }

    #[tokio::test]
    async fn speed_bench() {
        // field
        let (mut field_operation, field) = result_channel_pair(1);
        let mut field = NodeField::new("field".to_string(), field);

        // node a
        // default is 24,883,200 bytes length string.
        let (_, node_a, node_a_output) = get_string_node(
            "node_a".to_string(),
            "a_b_c_d_e_".to_string().repeat(2_488_320),
        );

        // display
        let (node_display_input, node_display, node_display_output) =
            get_string_node("display".to_string(), "display default".to_string());

        // channel to connect display node and operator.
        let (mut ch_call, dis_ch_out) = channel_pair(1);

        // ids for operation
        let node_a_id = node_a.get_id().clone();
        let node_display_id = node_display.get_id().clone();

        // node field
        let handle_field = tokio::spawn(async move {
            // connect display node to display operator
            node_display
                .connect_output(dis_ch_out, &node_display_output, &SocketId::new())
                .await
                .unwrap();

            // add nodes
            field.add_node(node_a);
            field.add_node(node_display);

            field.main_loop(10).await;
        });

        // operation to field. this works as a virtual frontend and command to display node.
        let handle_operator = tokio::spawn(async move {
            // connect display node to display operator
            let recv_result = ch_call.recv().await;
            if let Some(response) = recv_result {
                if let NodeResponse::CompatibleCheck { type_id, .. } = response {
                    assert_eq!(type_id, TypeId::of::<String>());
                    ch_call.send(NodeOrder::TypeConformed).await.unwrap();
                } else {
                    panic!();
                }
            } else {
                panic!();
            }

            // 1: "display default"
            ch_call.send(NodeOrder::Request { frame: 0 }).await.unwrap();

            match ch_call.recv().await.unwrap() {
                NodeResponse::Shared(shared) => {
                    assert_eq!(shared.downcast_ref::<String>().unwrap(), "display default");
                }
                _ => panic!(),
            }

            // a: "node_a"
            match field_operation
                .send(FrontToField::NodeConnect {
                    upstream_node_id: node_a_id.clone(),
                    upstream_node_socket_id: node_a_output.clone(),
                    downstream_node_id: node_display_id.clone(),
                    downstream_node_socket_id: node_display_input.clone(),
                })
                .await
                .unwrap()
            {
                Some(result) => match result {
                    FrontToFieldResult::NodeConnect(result) => {
                        assert_eq!(result, Ok(()));
                    }
                    _ => panic!(),
                },
                None => panic!(),
            }

            if let NodeResponse::DeleteCache = ch_call.recv().await.unwrap() {
            } else {
                panic!();
            }

            // time measurement
            let timer = std::time::Instant::now();

            for _ in 0..1000 {
                ch_call.send(NodeOrder::Request { frame: 1 }).await.unwrap();

                match ch_call.recv().await.unwrap() {
                    NodeResponse::Shared(shared) => {
                        assert_eq!(
                            shared.downcast_ref::<String>().unwrap(),
                            &"a_b_c_d_e_".to_string().repeat(2_488_320)
                        );
                    }
                    _ => panic!(),
                }
            }

            let elapsed = timer.elapsed();
            // print fps
            println!(
                "speed_bench> fps:           {} (assuming: raw 4K color image.)",
                1000.0 / elapsed.as_secs_f64()
            );

            // time measurement (unchecked)
            let timer = std::time::Instant::now();

            for _ in 0..1000 {
                ch_call.send(NodeOrder::Request { frame: 1 }).await.unwrap();

                match ch_call.recv().await.unwrap() {
                    NodeResponse::Shared(_) => {}
                    _ => panic!(),
                }
            }

            let elapsed = timer.elapsed();
            // print fps
            println!(
                "speed_bench> unchecked fps: {} (assuming: raw 4K color image.)",
                1000.0 / elapsed.as_secs_f64()
            );

            // shutdown
            match field_operation.send(FrontToField::Shutdown).await.unwrap() {
                Some(result) => match result {
                    FrontToFieldResult::Shutdown(result) => {
                        assert_eq!(result, Ok(()));
                    }
                    _ => panic!(),
                },
                None => panic!(),
            }

            // wait
            handle_field.await.unwrap();
        });

        handle_operator.await.unwrap();
    }

    #[cfg(test)]
    mod nodes {
        use std::{sync::Arc, vec};

        use crate::renderer::{
            node::NodeCoreCommon,
            types::{NodeId, SocketId},
        };

        use super::*;
        fn string_node_read(default: &String, _: &mut (), _: &Envelope, _: FrameCount) -> String {
            default.clone()
        }

        fn string_node_process(
            input: Arc<tokio::sync::Mutex<InputTree>>,
            _: &mut (),
            frame: FrameCount,
            _: &NodeId,
            _: &NodeName,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>> {
            let input = input.clone();
            Box::pin(async move {
                tokio::spawn(async move { input.lock().await.reef().get_clone(frame).await })
                    .await
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap()
                    .clone()
            })
        }

        fn string_node_pickup(output: &String) -> SharedAny {
            Box::new(output.clone())
        }

        pub fn get_string_node(
            name: String,
            default: String,
        ) -> (SocketId, Box<dyn NodeCoreCommon>, SocketId) {
            let input = Box::new(Input::new(default, (), Box::new(string_node_read)))
                as Box<dyn InputCommon>;
            let input_id = input.get_id().clone();

            let output = Output::new(Box::new(string_node_pickup));
            let output_id = output.get_id().clone();
            (
                input_id,
                Box::new(NodeCore::new(
                    name,
                    InputTree::Reef(input),
                    (),
                    Box::new(string_node_process),
                    OutputTree::new_reef(output),
                )),
                output_id,
            )
        }

        fn u64_node_read(_: &u64, _: &mut (), _: &Envelope, frame: FrameCount) -> u64 {
            frame as u64
        }

        fn u64_node_process(
            input: Arc<tokio::sync::Mutex<InputTree>>,
            _: &mut (),
            frame: FrameCount,
            _: &NodeId,
            _: &NodeName,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = u64> + Send>> {
            let input = input.clone();
            Box::pin(async move {
                tokio::spawn(async move { input.lock().await.reef().get_clone(frame).await })
                    .await
                    .unwrap()
                    .downcast_ref::<u64>()
                    .unwrap()
                    .clone()
            })
        }

        fn u64_node_pickup(output: &u64) -> SharedAny {
            Box::new(output.clone())
        }

        pub fn get_u64_node(
            name: String,
            default: u64,
        ) -> (SocketId, Box<dyn NodeCoreCommon>, SocketId, SocketId) {
            let input =
                Box::new(Input::new(default, (), Box::new(u64_node_read))) as Box<dyn InputCommon>;
            let input_id = input.get_id().clone();

            let output1 = Output::new(Box::new(u64_node_pickup));
            let output2 = Output::new(Box::new(u64_node_pickup));
            let output1_id = output1.get_id().clone();
            let output2_id = output2.get_id().clone();
            (
                input_id,
                Box::new(NodeCore::new(
                    name,
                    InputTree::Reef(input),
                    (),
                    Box::new(u64_node_process),
                    OutputTree::new_vec(vec![
                        OutputTree::new_reef(output1),
                        OutputTree::new_reef(output2),
                    ]),
                )),
                output1_id,
                output2_id,
            )
        }

        fn multiple_node_read_u64(default: &u64, _: &mut (), _: &Envelope, _: FrameCount) -> u64 {
            *default
        }

        fn multiple_node_read_string(
            default: &String,
            _: &mut (),
            _: &Envelope,
            _: FrameCount,
        ) -> String {
            default.clone()
        }

        fn multiple_node_process(
            input: Arc<tokio::sync::Mutex<InputTree>>,
            _: &mut (),
            frame: FrameCount,
            _: &NodeId,
            _: &NodeName,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>> {
            Box::pin(async move {
                let input_clone = input.clone();
                let i = tokio::spawn(async move {
                    input_clone.lock().await[0].reef().get_clone(frame).await
                })
                .await
                .unwrap()
                .downcast_ref::<u64>()
                .unwrap()
                .clone();
                let input_clone = input.clone();
                let s = tokio::spawn(async move {
                    input_clone.lock().await[1].reef().get_clone(frame).await
                })
                .await
                .unwrap()
                .downcast_ref::<String>()
                .unwrap()
                .clone();
                s.repeat(i as usize)
            })
        }

        fn multiple_node_pickup(output: &String) -> SharedAny {
            Box::new(output.clone())
        }

        pub fn get_multiple_node(
            name: String,
            default_u64: u64,
            default_string: String,
        ) -> (SocketId, SocketId, Box<dyn NodeCoreCommon>, SocketId) {
            let input_u64 = Box::new(Input::new(
                default_u64,
                (),
                Box::new(multiple_node_read_u64),
            )) as Box<dyn InputCommon>;
            let input_u64_id = input_u64.get_id().clone();

            let input_string = Box::new(Input::new(
                default_string,
                (),
                Box::new(multiple_node_read_string),
            )) as Box<dyn InputCommon>;
            let input_string_id = input_string.get_id().clone();

            let output = Output::new(Box::new(multiple_node_pickup));
            let output_id = output.get_id().clone();
            (
                input_u64_id,
                input_string_id,
                Box::new(NodeCore::new(
                    name,
                    InputTree::new_vec(vec![
                        InputTree::from(input_u64),
                        InputTree::from(input_string),
                    ]),
                    (),
                    Box::new(multiple_node_process),
                    OutputTree::new_reef(output),
                )),
                output_id,
            )
        }
    }
}
