use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::{Mutex, MutexGuard};

use crate::{
    node_core::StopNode,
    socket::{InputSocketCapsule, OutputSocketCapsule},
    FrameCount,
};

use super::{
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, UpdateInputDefaultError,
    },
    node_core::NodeCoreCommon,
    types::{FromToBinary, NodeId, SharedAny, SocketId},
};

pub struct NodeField {
    // information as a node
    node_id: NodeId,
    node_name: Mutex<String>,

    // nodes
    nodes: HashMap<NodeId, Arc<dyn NodeCoreCommon>>,

    // handle of node execution
    execution_handle: Option<tokio::task::JoinHandle<FrameCount>>,
    execution_stop_channel: Option<std::sync::mpsc::Sender<StopNode>>,
}

impl NodeField {
    pub fn new(name: &str) -> Self {
        NodeField {
            node_id: NodeId::new(),
            node_name: Mutex::new(name.to_string()),
            nodes: HashMap::new(),
            execution_handle: None,
            execution_stop_channel: None,
        }
    }

    // field operations

    pub fn add_node(&mut self, node: Arc<dyn NodeCoreCommon>) {
        self.nodes.insert(node.get_id(), node);
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
    async fn comprehensive_test() {
        // create field
        let mut field = NodeField::new("field");

        // create nodes
        let (node_a, node_a_input_id, node_a_output_id) =
            nodes::node_a::Builder {}.build_debug().await;
        let node_a_id = node_a.get_id();
        let (node_b, node_b_input_id, node_b_output_id) =
            nodes::node_b::Builder {}.build_debug().await;
        let node_b_id = node_b.get_id();
        let (node_c, node_c_input_id, node_c_output_id) =
            nodes::node_c::Builder {}.build_debug().await;
        let node_c_id = node_c.get_id();
        let (node_d, node_d_input_id, _) = nodes::node_d::Builder {}.build_debug().await;
        let node_d_id = node_d.get_id();

        // add nodes to field
        field.add_node(node_a);
        field.add_node(node_b);
        field.add_node(node_c);
        field.add_node(node_d);

        // check output
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_a_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_b_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_c_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_d_id).await.unwrap().downcast_ref::<i64>(), Some(&1));

        // change default value
        // field
        //     .update_input_default(node_a_id, node_a_input_id[0], Box::new(1 as i64))
        //     .await
        //     .unwrap();
        field
            .node_update_input_default(node_b_id, node_b_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_c_id, node_c_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_d_id, node_d_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_d_id, node_d_input_id[1], Box::new(1i64))
            .await
            .unwrap();

        // check output
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_a_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_b_id).await.unwrap().downcast_ref::<i64>(), Some(&2));
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_c_id).await.unwrap().downcast_ref::<i64>(), Some(&3));
        #[rustfmt::skip]
        assert_eq!(field.field_call(0, node_d_id).await.unwrap().downcast_ref::<i64>(), Some(&1));

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
        #[rustfmt::skip]
        assert_eq!(field.field_call(1, node_a_id).await.unwrap().downcast_ref::<i64>(), Some(&1));
        #[rustfmt::skip]
        assert_eq!(field.field_call(1, node_b_id).await.unwrap().downcast_ref::<i64>(), Some(&2));
        #[rustfmt::skip]
        assert_eq!(field.field_call(1, node_c_id).await.unwrap().downcast_ref::<i64>(), Some(&3));
        #[rustfmt::skip]
        assert_eq!(field.field_call(1, node_d_id).await.unwrap().downcast_ref::<i64>(), Some(&8));

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
        let mut field = NodeField::new("field");

        // create nodes
        let (node_a, node_a_input_id, node_a_output_id) =
            nodes::node_a::Builder {}.build_debug().await;
        let node_a_id = node_a.get_id();
        let (node_b, node_b_input_id, node_b_output_id) =
            nodes::node_b::Builder {}.build_debug().await;
        let node_b_id = node_b.get_id();
        let (node_c, node_c_input_id, node_c_output_id) =
            nodes::node_c::Builder {}.build_debug().await;
        let node_c_id = node_c.get_id();
        let (node_d, node_d_input_id, node_d_output_id) =
            nodes::node_d::Builder {}.build_debug().await;
        let node_d_id = node_d.get_id();

        // add nodes to field
        field.add_node(node_a);
        field.add_node(node_b);
        field.add_node(node_c);
        field.add_node(node_d);

        // change default value
        field
            .node_update_input_default(node_b_id, node_b_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_c_id, node_c_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_d_id, node_d_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_d_id, node_d_input_id[1], Box::new(1i64))
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
            let _ = field
                .field_call(i, node_d_id)
                .await
                .unwrap()
                .downcast_ref::<i64>();
        }

        println!("1000 times took:      {:?}", timer.elapsed());
        println!("average:              {:?}", timer.elapsed() / 1000);
        println!("average per node:     {:?}", timer.elapsed() / 4000);
        println!(
            "fps per node will be: {:?}",
            1.0 / (timer.elapsed().as_secs_f64() / 4000.0)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 20)]
    async fn node_execution_test() {
        // create field
        let mut field = NodeField::new("field");

        // create nodes
        let (node_a, node_a_input_id, node_a_output_id) =
            nodes::node_a::Builder {}.build_debug().await;
        let node_a_id = node_a.get_id();
        let (node_b, node_b_input_id, node_b_output_id) =
            nodes::node_b::Builder {}.build_debug().await;
        let node_b_id = node_b.get_id();
        let (node_c, node_c_input_id, node_c_output_id) =
            nodes::node_c::Builder {}.build_debug().await;
        let node_c_id = node_c.get_id();
        let (node_d, node_d_input_id, node_d_output_id) =
            nodes::node_d::Builder {}.build_debug().await;
        let node_d_id = node_d.get_id();

        // add nodes to field
        field.add_node(node_a);
        field.add_node(node_b);
        field.add_node(node_c);
        field.add_node(node_d);

        // change default value
        field
            .node_update_input_default(node_b_id, node_b_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_c_id, node_c_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_d_id, node_d_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        field
            .node_update_input_default(node_d_id, node_d_input_id[1], Box::new(1i64))
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

        // play
        field.field_play(0, node_d_id).await.unwrap();

        // wait
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // stop
        let frame = field.field_stop().await;
        println!("Execution stopped at frame: {:?}", frame);
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
        // |  /
        // | /
        // |/
        // D

        // A: u64 input (output number same as frame count)
        // B: u64 multiply 2
        // C: u64 multiply 3
        // D: u64 b^c

        pub mod node_a {
            use crate::{
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::Arc;

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                fn name(&self) -> &'static str {
                    "A"
                }

                async fn build(&self) -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("INPUT", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn build_debug(
                    &self,
                ) -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("INPUT", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id = input.input_1.get_id();
                    let output = output_1::build(node.clone()).to_capsule();
                    let output_id = output.socket_id();
                    let output = OutputTree::Socket(output);

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id], vec![output_id])
                }

                async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
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
                async fn get_socket(&self, id: SocketId) -> Option<InputSocketCapsule> {
                    if id == self.input_1.get_id() {
                        Some(self.input_1.make_capsule())
                    } else {
                        None
                    }
                }

                fn get_all_socket(&self) -> Vec<InputSocketCapsule> {
                    vec![self.input_1.make_capsule()]
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    Self { input_1 }
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
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                    InputSocket::new(
                        "input",
                        node,
                        "",
                        None,
                        "Frame Count",
                        Some(Envelope::new_pass_through()),
                        (),
                        Box::new(read),
                        Some(Envelope::new_pass_through()),
                    )
                }

                // read from default value or envelope
                fn read(
                    _: Option<&Default>,
                    envelope: Option<&Envelope>,
                    _: &mut (),
                    frame: FrameCount,
                ) -> SocketType {
                    envelope.unwrap().value(frame as f64) as i64
                }
            }

            // Output

            fn give_output_tree(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
                OutputTree::Socket(output_1::build(node).to_capsule())
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::Arc;

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                fn name(&self) -> &'static str {
                    "*1"
                }

                async fn build(&self) -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("*2", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn build_debug(
                    &self,
                ) -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("*2", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id = input.input_1.get_id();
                    let output = output_1::build(node.clone()).to_capsule();
                    let output_id = output.socket_id();
                    let output = OutputTree::Socket(output);

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id], vec![output_id])
                }

                async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
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
                async fn get_socket(&self, id: SocketId) -> Option<InputSocketCapsule> {
                    if id == self.input_1.get_id() {
                        Some(self.input_1.make_capsule())
                    } else {
                        None
                    }
                }

                fn get_all_socket(&self) -> Vec<InputSocketCapsule> {
                    vec![self.input_1.make_capsule()]
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    Self { input_1 }
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
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                    InputSocket::new(
                        "input",
                        node,
                        "i64",
                        Some(0),
                        "",
                        None,
                        (),
                        Box::new(read),
                        Some(Envelope::new_pass_through()),
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
                OutputTree::Socket(output_1::build(node).to_capsule())
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::Arc;

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                fn name(&self) -> &'static str {
                    "*3"
                }

                async fn build(&self) -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("*3", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn build_debug(
                    &self,
                ) -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("*3", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id = input.input_1.get_id();
                    let output = output_1::build(node.clone()).to_capsule();
                    let output_id = output.socket_id();
                    let output = OutputTree::Socket(output);

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id], vec![output_id])
                }

                async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
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
                async fn get_socket(&self, id: SocketId) -> Option<InputSocketCapsule> {
                    if id == self.input_1.get_id() {
                        Some(self.input_1.make_capsule())
                    } else {
                        None
                    }
                }

                fn get_all_socket(&self) -> Vec<InputSocketCapsule> {
                    vec![self.input_1.make_capsule()]
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    Self { input_1 }
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
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                    InputSocket::new(
                        "input",
                        node,
                        "i64",
                        Some(0),
                        "",
                        None,
                        (),
                        Box::new(read),
                        Some(Envelope::new_pass_through()),
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
                OutputTree::Socket(output_1::build(node).to_capsule())
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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
                framework::NodeFramework,
                node_core::{NodeCore, NodeCoreCommon},
                socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
                types::{NodeName, SocketId},
                FrameCount,
            };
            use envelope::Envelope;
            use std::sync::Arc;

            // Types of Node

            type NodeMemory = ();
            type NodeOutput = i64;

            // Node

            pub struct Builder;

            #[async_trait::async_trait]
            impl NodeFramework for Builder {
                fn name(&self) -> &'static str {
                    "PAW"
                }

                async fn build(&self) -> Arc<dyn NodeCoreCommon> {
                    let node = Arc::new(NodeCore::new("PAW", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let output = give_output_tree(node.clone());

                    node.set_input(input).await;
                    node.set_output(output).await;

                    node
                }

                async fn build_debug(
                    &self,
                ) -> (
                    Arc<dyn NodeCoreCommon>,
                    Vec<crate::types::SocketId>,
                    Vec<crate::types::SocketId>,
                ) {
                    let node = Arc::new(NodeCore::new("PAW", (), Box::new(node_main_process)));

                    let input = Inputs::new(node.clone());
                    let input_id_1 = input.input_1.get_id();
                    let input_id_2 = input.input_2.get_id();
                    let output = output_1::build(node.clone()).to_capsule();
                    let output_id = output.socket_id();
                    let output = OutputTree::Socket(output);

                    node.set_input(input).await;
                    node.set_output(output).await;

                    (node, vec![input_id_1, input_id_2], vec![output_id])
                }

                async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
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
                    let input_1 = input.input_1.clone();
                    let input_2 = input.input_2.clone();
                    let handle_b = tokio::spawn(async move { input_1.get(frame).await });
                    let handle_c = tokio::spawn(async move { input_2.get(frame).await });
                    let b = handle_b.await.unwrap();
                    let c = handle_c.await.unwrap();
                    b.overflowing_pow(c as u32).0
                })
            }

            // Input

            struct Inputs {
                input_1: Arc<input_1::Socket>,
                input_2: Arc<input_1::Socket>,
            }

            #[async_trait::async_trait]
            impl InputGroup for Inputs {
                async fn get_socket(&self, id: SocketId) -> Option<InputSocketCapsule> {
                    if id == self.input_1.get_id() {
                        Some(self.input_1.make_capsule())
                    } else if id == self.input_2.get_id() {
                        Some(self.input_2.make_capsule())
                    } else {
                        None
                    }
                }

                fn get_all_socket(&self) -> Vec<InputSocketCapsule> {
                    vec![self.input_1.make_capsule(), self.input_2.make_capsule()]
                }
            }

            impl Inputs {
                fn new(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Self {
                    let input_1 = input_1::build(node.clone());
                    let input_2 = input_1::build(node.clone());
                    Self { input_1, input_2 }
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
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                    InputSocket::new(
                        "input",
                        node,
                        "i64",
                        Some(0),
                        "",
                        None,
                        (),
                        Box::new(read),
                        Some(Envelope::new_pass_through()),
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
                OutputTree::Socket(output_1::build(node).to_capsule())
            }

            mod output_1 {
                use super::*;

                // types
                type SocketType = i64;
                pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

                // build socket
                pub fn build(node: Arc<NodeCore<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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
