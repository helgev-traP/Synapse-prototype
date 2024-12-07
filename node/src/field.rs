use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};

use tokio::sync::{Mutex, MutexGuard};

use crate::{
    socket::{OutputTrait, WeakInputSocket, WeakOutputSocket},
    FrameCount,
};

use super::{
    channel::FieldChannel,
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, UpdateInputDefaultError,
    },
    node_core::NodeCoreCommon,
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
    node_name: Arc<Mutex<String>>,
    nodes: HashMap<NodeId, Arc<dyn NodeCoreCommon>>,
    channel_front: tokio::sync::Mutex<FieldChannel>,
}

impl NodeField {
    pub fn new(name: String, channel: FieldChannel) -> Self {
        NodeField {
            node_id: NodeId::new(),
            node_name: Arc::new(Mutex::new(name)),
            nodes: HashMap::new(),
            channel_front: tokio::sync::Mutex::new(channel),
        }
    }

    // field operations

    pub fn add_node(&mut self, node: Arc<dyn NodeCoreCommon>) {
        self.nodes.insert(node.get_id().clone(), node);
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
            if *id != self.nodes.get(id).unwrap().get_id() {
                return false;
            }
        }
        return true;
    }

    pub async fn ensure_consistency(&mut self) {
        let mut ids = HashSet::new();
        for (id, node) in self.nodes.iter() {
            if *id != node.get_id() {
                ids.insert(id.clone());
            }
        }
        for id in ids {
            let node = self.nodes.remove(&id).unwrap();
            let id = node.get_id().clone();
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
            .map(|socket| socket.week())
        else {
            return Err(NodeDisconnectError::SocketIdNotFound);
        };

        // disconnect
        downstream_socket.upgrade().unwrap().disconnect().await
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
            .map(|socket| socket.week())
        else {
            return Err(NodeDisconnectError::SocketIdNotFound);
        };

        // check connection
        let upstream_socket = upstream_socket.upgrade().unwrap();
        let downstream_socket = downstream_socket.upgrade().unwrap();

        if upstream_socket
            .get_downstream_ids()
            .await
            .contains(&downstream_socket.get_id())
        {
            // disconnect
            upstream_socket.disconnect(downstream_socket.get_id()).await
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
            .map(|socket| socket.week())
        else {
            return Err(NodeConnectionCheckError::SocketIdNotFound);
        };

        // check connection
        let upstream_socket = upstream_socket.upgrade().unwrap();
        let downstream_socket = downstream_socket.upgrade().unwrap();

        if upstream_socket
            .get_downstream_ids()
            .await
            .contains(&downstream_socket.get_id())
        {
            Ok(())
        } else {
            Err(NodeConnectionCheckError::NotConnected)
        }
    }

    /*
    pub async fn node_disconnect_all(
        &mut self,
        node_id: &NodeId,
        socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        todo!()
    }
    */

    // update input default of node
    pub async fn update_input_default(
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

    /*
    pub async fn main_oneshot(&self) {
        let mut handles = Vec::new();

        for (_, node) in self.nodes.iter() {
            // execute node's main
            let node = node.clone();

            // task spawn
            handles.push(tokio::spawn(async move {
                let mut node = node.lock().await;
                (*node).main().await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }
    */

    pub async fn one_shot(&self, frame: FrameCount, node_id: NodeId) -> Option<Arc<SharedAny>> {
        if let Some(node) = self.nodes.get(&node_id) {
            Some(node.call(frame).await)
        } else {
            None
        }
    }

    pub async fn play(&self) {
        todo!()
    }
}

#[async_trait::async_trait]
impl NodeCoreCommon for NodeField {
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

    async fn get_input_socket(&self, socket_id: SocketId) -> Option<WeakInputSocket> {
        todo!()
    }

    async fn get_output_socket(&self, socket_id: SocketId) -> Option<WeakOutputSocket> {
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

    async fn play(&self, frame: FrameCount) {
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
    use crate::{channel::result_channel_pair, framework::NodeFramework};

    use super::*;

    #[tokio::test]
    async fn hash_map_ensure_consistency_test() {
        // todo
    }

    #[tokio::test]
    async fn test() {
        // create field
        let mut field = NodeField::new("field".to_string(), result_channel_pair(1).0);

        // create nodes
        let (node_a, node_a_input_id, node_a_output_id) = nodes::node_a::Builder::new_debug().await;
        let node_a_id = node_a.get_id();
        let (node_b, node_b_input_id, node_b_output_id) = nodes::node_b::Builder::new_debug().await;
        let node_b_id = node_b.get_id();
        let (node_c, node_c_input_id, node_c_output_id) = nodes::node_c::Builder::new_debug().await;
        let node_c_id = node_c.get_id();
        let (node_d, node_d_input_id, _) = nodes::node_d::Builder::new_debug().await;
        let node_d_id = node_d.get_id();

        // add nodes to field
        field.add_node(node_a);
        field.add_node(node_b);
        field.add_node(node_c);
        field.add_node(node_d);

        // check output
        assert_eq!(
            field
                .one_shot(0, node_a_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&0)
        );
        assert_eq!(
            field
                .one_shot(1, node_b_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&0)
        );
        assert_eq!(
            field
                .one_shot(2, node_c_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&0)
        );
        assert_eq!(
            field
                .one_shot(3, node_d_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&1)
        );

        // change default value
        field
            .update_input_default(node_a_id, node_a_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_b_id, node_b_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_c_id, node_c_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_d_id, node_d_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_d_id, node_d_input_id[1], Box::new(1 as i64))
            .await
            .unwrap();

        // check output
        assert_eq!(
            field
                .one_shot(4, node_a_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&1)
        );
        assert_eq!(
            field
                .one_shot(5, node_b_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&2)
        );
        assert_eq!(
            field
                .one_shot(6, node_c_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&3)
        );
        assert_eq!(
            field
                .one_shot(7, node_d_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&1)
        );

        // connect nodes
        field
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_b_id,
                node_b_input_id[0],
            )
            .await
            .unwrap();
        field
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_c_id,
                node_c_input_id[0],
            )
            .await
            .unwrap();
        field
            .node_connect(
                node_b_id,
                node_b_output_id[0],
                node_d_id,
                node_d_input_id[0],
            )
            .await
            .unwrap();
        field
            .node_connect(
                node_c_id,
                node_c_output_id[0],
                node_d_id,
                node_d_input_id[1],
            )
            .await
            .unwrap();

        // check output
        assert_eq!(
            field
                .one_shot(8, node_a_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&1)
        );
        assert_eq!(
            field
                .one_shot(9, node_b_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&2)
        );
        assert_eq!(
            field
                .one_shot(10, node_c_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&3)
        );
        assert_eq!(
            field
                .one_shot(11, node_d_id)
                .await
                .unwrap()
                .downcast_ref::<i64>(),
            Some(&8)
        );

        // check connection
        assert_eq!(
            field
                .check_connection(
                    node_a_id,
                    node_a_output_id[0],
                    node_b_id,
                    node_b_input_id[0]
                )
                .await,
            Ok(())
        );
        assert_eq!(
            field
                .check_connection(
                    node_a_id,
                    node_a_output_id[0],
                    node_c_id,
                    node_c_input_id[0]
                )
                .await,
            Ok(())
        );
        assert_eq!(
            field
                .check_connection(
                    node_b_id,
                    node_b_output_id[0],
                    node_d_id,
                    node_d_input_id[0]
                )
                .await,
            Ok(())
        );
        assert_eq!(
            field
                .check_connection(
                    node_c_id,
                    node_c_output_id[0],
                    node_d_id,
                    node_d_input_id[1]
                )
                .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn measure_transfer_overhead() {
        // create field
        let mut field = NodeField::new("field".to_string(), result_channel_pair(1).0);

        // create nodes
        let (node_a, node_a_input_id, node_a_output_id) = nodes::node_a::Builder::new_debug().await;
        let node_a_id = node_a.get_id();
        let (node_b, node_b_input_id, node_b_output_id) = nodes::node_b::Builder::new_debug().await;
        let node_b_id = node_b.get_id();
        let (node_c, node_c_input_id, node_c_output_id) = nodes::node_c::Builder::new_debug().await;
        let node_c_id = node_c.get_id();
        let (node_d, node_d_input_id, node_d_output_id) = nodes::node_d::Builder::new_debug().await;
        let node_d_id = node_d.get_id();

        // add nodes to field
        field.add_node(node_a);
        field.add_node(node_b);
        field.add_node(node_c);
        field.add_node(node_d);

        // change default value
        field
            .update_input_default(node_a_id, node_a_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_b_id, node_b_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_c_id, node_c_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_d_id, node_d_input_id[0], Box::new(1 as i64))
            .await
            .unwrap();
        field
            .update_input_default(node_d_id, node_d_input_id[1], Box::new(1 as i64))
            .await
            .unwrap();

        // connect nodes
        field
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_b_id,
                node_b_input_id[0],
            )
            .await
            .unwrap();
        field
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_c_id,
                node_c_input_id[0],
            )
            .await
            .unwrap();
        field
            .node_connect(
                node_b_id,
                node_b_output_id[0],
                node_d_id,
                node_d_input_id[0],
            )
            .await
            .unwrap();
        field
            .node_connect(
                node_c_id,
                node_c_output_id[0],
                node_d_id,
                node_d_input_id[1],
            )
            .await
            .unwrap();

        let timer = std::time::Instant::now();

        // check output
        for i in 0..1000 {
            assert_eq!(
                field
                    .one_shot(i, node_d_id)
                    .await
                    .unwrap()
                    .downcast_ref::<i64>(),
                Some(&8)
            );
        }

        println!("1000 times took:      {:?}", timer.elapsed());
        println!("average:              {:?}", timer.elapsed() / 1000);
        println!("average per node:     {:?}", timer.elapsed() / 4000);
        println!(
            "fps per node will be: {:?}",
            1.0 / (timer.elapsed().as_secs_f64() / 4000.0)
        );
    }

    #[tokio::test]
    async fn speed_bench_with_4k_data() {}

    #[cfg(test)]
    mod nodes {
        // Nodes:
        // A
        // |\
        // | \
        // |  \
        // B   C
        // |   /
        // |  /
        // | /
        // D

        // A: u64 input
        // B: u64 multiply 2
        // C: u64 multiply 3
        // D: u64 b^c

        pub mod node_a {
            use crate::{
                field::OutputTrait,
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{
                    InputGroup, InputSocket, InputTrait, OutputSocket,
                    OutputTree, WeakInputSocket,
                },
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::{Arc, Weak};

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                async fn new() -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("A", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn new_debug() -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("A", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id = input.input_1.get_id();
                    let output = output_1::build(node.clone());
                    let output_id = output.get_id();
                    let output = OutputTree::Socket(Arc::new(output));

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id], vec![output_id])
                }

                async fn build_from_binary(_: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
                    todo!()
                }
            }

            fn node_main_process<'a>(
                _: &'a NodeName,
                input: &'a Inputs,
                _: &'a mut (),
                frame: FrameCount,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>>
            {
                Box::pin(async move { input.input_1.get(frame).await })
            }

            // Input

            struct Inputs {
                input_1: Arc<input_1::Socket>,
            }

            #[async_trait::async_trait]
            impl InputGroup for Inputs {
                async fn get_socket(&self, id: SocketId) -> Option<WeakInputSocket> {
                    if id == self.input_1.get_id() {
                        Some(Arc::downgrade(&self.input_1) as WeakInputSocket)
                    } else {
                        None
                    }
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    Self {
                        input_1: Arc::new(input_1),
                    }
                }
            }

            // Input Sockets

            mod input_1 {

                use super::*;

                // types
                type Default = i64;
                type Memory = ();
                type SocketType = i64;
                pub type Socket =
                    InputSocket<Default, Memory, SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    InputSocket::new(
                        "input",
                        node,
                        "i64",
                        Some(0),
                        "",
                        None,
                        (),
                        Box::new(read),
                        Envelope::new_pass_through(),
                    )
                }

                // read from default value or envelope
                fn read(
                    default: Option<&Default>,
                    _: Option<&Envelope>,
                    _: &mut (),
                    _: FrameCount,
                ) -> SocketType {
                    *default.unwrap()
                }
            }

            // Output

            fn give_output_tree(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
                OutputTree::Socket(Arc::new(output_1::build(node)))
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    OutputSocket::new("output", Box::new(pickup), node)
                }

                // pick up from the output of node main process
                fn pickup(s: &NodeOutput) -> SocketType {
                    s.clone()
                }
            }
        }

        pub mod node_b {
            use crate::{
                field::OutputTrait,
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{
                    InputGroup, InputSocket, InputTrait, OutputSocket,
                    OutputTree, WeakInputSocket,
                },
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::{Arc, Weak};

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                async fn new() -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn new_debug() -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id = input.input_1.get_id();
                    let output = output_1::build(node.clone());
                    let output_id = output.get_id();
                    let output = OutputTree::Socket(Arc::new(output));

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id], vec![output_id])
                }

                async fn build_from_binary(_: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
                    todo!()
                }
            }

            fn node_main_process<'a>(
                _: &'a NodeName,
                input: &'a Inputs,
                _: &'a mut (),
                frame: FrameCount,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>>
            {
                Box::pin(async move { input.input_1.get(frame).await * 2 })
            }

            // Input

            struct Inputs {
                input_1: Arc<input_1::Socket>,
            }

            #[async_trait::async_trait]
            impl InputGroup for Inputs {
                async fn get_socket(&self, id: SocketId) -> Option<Weak<dyn InputTrait>> {
                    if id == self.input_1.get_id() {
                        Some(Arc::downgrade(&self.input_1) as Weak<dyn InputTrait>)
                    } else {
                        None
                    }
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    Self {
                        input_1: Arc::new(input_1),
                    }
                }
            }

            // Input Sockets

            mod input_1 {
                use super::*;

                // types
                type Default = i64;
                type Memory = ();
                type SocketType = i64;
                pub type Socket =
                    InnerInputSocket<Default, Memory, SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    InnerInputSocket::new(
                        "input",
                        node,
                        "i64",
                        Some(0),
                        "",
                        None,
                        (),
                        Box::new(read),
                        Envelope::new_pass_through(),
                    )
                }

                // read from default value or envelope
                fn read(
                    default: Option<&Default>,
                    _: Option<&Envelope>,
                    _: &mut (),
                    _: FrameCount,
                ) -> SocketType {
                    *default.unwrap()
                }
            }

            // Output

            fn give_output_tree(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
                OutputTree::Socket(Arc::new(output_1::build(node)))
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    OutputSocket::new("output", Box::new(pickup), node)
                }

                // pick up from the output of node main process
                fn pickup(s: &NodeOutput) -> SocketType {
                    s.clone()
                }
            }
        }

        pub mod node_c {
            use crate::{
                field::OutputTrait,
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{
                    InputGroup, InputSocket, InputTrait, OutputSocket,
                    OutputTree, WeakInputSocket,
                },
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::{Arc, Weak};

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                async fn new() -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn new_debug() -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id = input.input_1.get_id();
                    let output = output_1::build(node.clone());
                    let output_id = output.get_id();
                    let output = OutputTree::Socket(Arc::new(output));

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id], vec![output_id])
                }

                async fn build_from_binary(_: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
                    todo!()
                }
            }

            fn node_main_process<'a>(
                _: &'a NodeName,
                input: &'a Inputs,
                _: &'a mut (),
                frame: FrameCount,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>>
            {
                Box::pin(async move { input.input_1.get(frame).await * 3 })
            }

            // Input

            struct Inputs {
                input_1: Arc<input_1::Socket>,
            }

            #[async_trait::async_trait]
            impl InputGroup for Inputs {
                async fn get_socket(&self, id: SocketId) -> Option<Weak<dyn InputTrait>> {
                    if id == self.input_1.get_id() {
                        Some(Arc::downgrade(&self.input_1) as Weak<dyn InputTrait>)
                    } else {
                        None
                    }
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    Self {
                        input_1: Arc::new(input_1),
                    }
                }
            }

            // Input Sockets

            mod input_1 {
                use super::*;

                // types
                type Default = i64;
                type Memory = ();
                type SocketType = i64;
                pub type Socket =
                    InnerInputSocket<Default, Memory, SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    InnerInputSocket::new(
                        "input",
                        node,
                        "i64",
                        Some(0),
                        "",
                        None,
                        (),
                        Box::new(read),
                        Envelope::new_pass_through(),
                    )
                }

                // read from default value or envelope
                fn read(
                    default: Option<&Default>,
                    _: Option<&Envelope>,
                    _: &mut (),
                    _: FrameCount,
                ) -> SocketType {
                    *default.unwrap()
                }
            }

            // Output

            fn give_output_tree(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
                OutputTree::Socket(Arc::new(output_1::build(node)))
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    OutputSocket::new("output", Box::new(pickup), node)
                }

                // pick up from the output of node main process
                fn pickup(s: &NodeOutput) -> SocketType {
                    s.clone()
                }
            }
        }

        pub mod node_d {
            use crate::{
                field::OutputTrait,
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{
                    InputGroup, InputSocket, InputTrait, OutputSocket,
                    OutputTree, WeakInputSocket,
                },
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::{Arc, Weak};

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                async fn new() -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn new_debug() -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id_1 = input.input_1.get_id();
                    let input_id_2 = input.input_2.get_id();
                    let output = output_1::build(node.clone());
                    let output_id = output.get_id();
                    let output = OutputTree::Socket(Arc::new(output));

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id_1, input_id_2], vec![output_id])
                }

                async fn build_from_binary(_: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
                    todo!()
                }
            }

            fn node_main_process<'a>(
                _: &'a NodeName,
                input: &'a Inputs,
                _: &'a mut (),
                frame: FrameCount,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>>
            {
                Box::pin(async move {
                    let b = input.input_1.get(frame).await;
                    let c = input.input_2.get(frame).await;
                    b.pow(c as u32)
                })
            }

            // Input

            struct Inputs {
                input_1: Arc<input_1::Socket>,
                input_2: Arc<input_1::Socket>,
            }

            #[async_trait::async_trait]
            impl InputGroup for Inputs {
                async fn get_socket(&self, id: SocketId) -> Option<Weak<dyn InputTrait>> {
                    if id == self.input_1.get_id() {
                        Some(Arc::downgrade(&self.input_1) as Weak<dyn InputTrait>)
                    } else if id == self.input_2.get_id() {
                        Some(Arc::downgrade(&self.input_2) as Weak<dyn InputTrait>)
                    } else {
                        None
                    }
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    let input_2 = input_1::build(node.clone());
                    Self {
                        input_1: Arc::new(input_1),
                        input_2: Arc::new(input_2),
                    }
                }
            }

            // Input Sockets

            mod input_1 {
                use super::*;

                // types
                type Default = i64;
                type Memory = ();
                type SocketType = i64;
                pub type Socket =
                    InnerInputSocket<Default, Memory, SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    InnerInputSocket::new(
                        "input",
                        node,
                        "i64",
                        Some(0),
                        "",
                        None,
                        (),
                        Box::new(read),
                        Envelope::new_pass_through(),
                    )
                }

                // read from default value or envelope
                fn read(
                    default: Option<&Default>,
                    _: Option<&Envelope>,
                    _: &mut (),
                    _: FrameCount,
                ) -> SocketType {
                    *default.unwrap()
                }
            }

            // Output

            fn give_output_tree(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
                OutputTree::Socket(Arc::new(output_1::build(node)))
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Socket {
                    OutputSocket::new("output", Box::new(pickup), node)
                }

                // pick up from the output of node main process
                fn pickup(s: &NodeOutput) -> SocketType {
                    s.clone()
                }
            }
        }
    }
}
