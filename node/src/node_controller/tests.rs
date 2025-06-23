use crate::plugin::Plugin;

use super::*;

#[tokio::test]
async fn hash_map_ensure_consistency_test() {
    // todo
}

#[tokio::test]
    #[rustfmt::skip]
    async fn comprehensive_test() {
        // create controller
        let mut controller = NodeController::new("controller");

        // create nodes
        // a
        let node_a = nodes::node_a::PluginA {}.build().await;
        let node_a_output_id =
            node_a.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_a_id = node_a.get_id();

        // b
        let node_b = nodes::node_b::PluginB {}.build().await;
        let node_b_input_id =
            node_b.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_b_output_id =
            node_b.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_b_id = node_b.get_id();

        // c
        let node_c = nodes::node_c::PluginC {}.build().await;
        let node_c_input_id =
            node_c.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_c_output_id =
            node_c.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_c_id = node_c.get_id();

        // d
        let node_d = nodes::node_d::PluginD {}.build().await;
        let node_d_input_id =
            node_d.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_d_id = node_d.get_id();

        // add nodes to controller
        controller.add_node(node_a);
        controller.add_node(node_b);
        controller.add_node(node_c);
        controller.add_node(node_d);

        // check output
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_a_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_b_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_c_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_d_id).await.unwrap().downcast_ref::<i64>(), Some(&1));

        // change default value
        // controller
        //     .update_input_default(node_a_id, node_a_input_id[0], Box::new(1 as i64))
        //     .await
        //     .unwrap();
        controller
            .node_update_input_default(node_b_id, node_b_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_c_id, node_c_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_d_id, node_d_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_d_id, node_d_input_id[1], Box::new(1i64))
            .await
            .unwrap();

        // check output
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_a_id).await.unwrap().downcast_ref::<i64>(), Some(&0));
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_b_id).await.unwrap().downcast_ref::<i64>(), Some(&2));
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_c_id).await.unwrap().downcast_ref::<i64>(), Some(&3));
        #[rustfmt::skip]
        assert_eq!(controller.controller_call(0, node_d_id).await.unwrap().downcast_ref::<i64>(), Some(&1));

        // connect nodes
        controller
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_b_id,
                node_b_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_c_id,
                node_c_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_b_id,
                node_b_output_id[0],
                node_d_id,
                node_d_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_c_id,
                node_c_output_id[0],
                node_d_id,
                node_d_input_id[1],
            )
            .await
            .unwrap();

        // check output
        assert_eq!(controller.controller_call(1, node_a_id).await.unwrap().downcast_ref::<i64>(), Some(&1));
        assert_eq!(controller.controller_call(1, node_b_id).await.unwrap().downcast_ref::<i64>(), Some(&2));
        assert_eq!(controller.controller_call(1, node_c_id).await.unwrap().downcast_ref::<i64>(), Some(&3));
        assert_eq!(controller.controller_call(1, node_d_id).await.unwrap().downcast_ref::<i64>(), Some(&8));

        // check connection
        assert_eq!(
            controller
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
            controller
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
            controller
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
            controller
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
    #[rustfmt::skip]
    async fn measure_transfer_overhead() {
        // create controller
        let mut controller = NodeController::new("controller");

        // create
        let node_a = nodes::node_a::PluginA {}.build().await;
        let node_a_output_id =
            node_a.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_a_id = node_a.get_id();

        let node_b = nodes::node_b::PluginB {}.build().await;
        let node_b_input_id =
            node_b.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_b_output_id =
            node_b.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_b_id = node_b.get_id();

        let node_c = nodes::node_c::PluginC {}.build().await;
        let node_c_input_id =
            node_c.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_c_output_id =
            node_c.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_c_id = node_c.get_id();

        let node_d = nodes::node_d::PluginD {}.build().await;
        let node_d_input_id =
            node_d.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_d_id = node_d.get_id();

        // add nodes to controller
        controller.add_node(node_a);
        controller.add_node(node_b);
        controller.add_node(node_c);
        controller.add_node(node_d);

        // change default value
        controller
            .node_update_input_default(node_b_id, node_b_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_c_id, node_c_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_d_id, node_d_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_d_id, node_d_input_id[1], Box::new(1i64))
            .await
            .unwrap();

        // connect nodes
        controller
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_b_id,
                node_b_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_c_id,
                node_c_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_b_id,
                node_b_output_id[0],
                node_d_id,
                node_d_input_id[0],
            )
            .await
            .unwrap();
        controller
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
            let _ = controller
                .controller_call(i, node_d_id)
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
    #[rustfmt::skip]
    async fn node_execution_test() {
        // create controller
        let mut controller = NodeController::new("controller");

        // create nodes
        let node_a = nodes::node_a::PluginA {}.build().await;
        let node_a_output_id =
            node_a.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_a_id = node_a.get_id();

        let node_b = nodes::node_b::PluginB {}.build().await;
        let node_b_input_id =
            node_b.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_b_output_id =
            node_b.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_b_id = node_b.get_id();

        let node_c = nodes::node_c::PluginC {}.build().await;
        let node_c_input_id =
            node_c.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_c_output_id =
            node_c.get_all_output_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_c_id = node_c.get_id();

        let node_d = nodes::node_d::PluginD {}.build().await;
        let node_d_input_id =
            node_d.get_all_input_socket().await.iter().map(|s| s.socket_id()).collect::<Vec<_>>();
        let node_d_id = node_d.get_id();

        // add nodes to controller
        controller.add_node(node_a);
        controller.add_node(node_b);
        controller.add_node(node_c);
        controller.add_node(node_d);

        // change default value
        controller
            .node_update_input_default(node_b_id, node_b_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_c_id, node_c_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_d_id, node_d_input_id[0], Box::new(1i64))
            .await
            .unwrap();
        controller
            .node_update_input_default(node_d_id, node_d_input_id[1], Box::new(1i64))
            .await
            .unwrap();

        // connect nodes
        controller
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_b_id,
                node_b_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_a_id,
                node_a_output_id[0],
                node_c_id,
                node_c_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_b_id,
                node_b_output_id[0],
                node_d_id,
                node_d_input_id[0],
            )
            .await
            .unwrap();
        controller
            .node_connect(
                node_c_id,
                node_c_output_id[0],
                node_d_id,
                node_d_input_id[1],
            )
            .await
            .unwrap();

        // play
        controller.controller_play(0, node_d_id).await.unwrap();

        // wait
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // stop
        let frame = controller.controller_stop().await;
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
            node::{Node, NodeCommon},
            plugin::Plugin,
            socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
            types::{NodeName, PluginId, SocketId},
            FrameCount,
        };
        use envelope::Envelope;
        use std::sync::{Arc, Weak};

        // Types of Node

        type NodeMemory = ();
        type NodeOutput = i64;

        // Node

        pub struct PluginA;

        #[async_trait::async_trait]
        impl Plugin for PluginA {
            fn name(&self) -> &'static str {
                "A"
            }

            fn plugin_id(&self) -> PluginId {
                PluginId::from_string("fe600d8d-465c-b0fb-2dde-337e65598ee3")
            }

            async fn build(&self) -> Arc<dyn NodeCommon> {
                (Node::new(
                    "INPUT",
                    Inputs::new,
                    (),
                    Box::new(node_main_process),
                    give_output_tree,
                )) as _
            }

            async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCommon>, &[u8]) {
                todo!()
            }
        }

        fn node_main_process<'a>(
            _: &'a NodeName,
            input: &'a Inputs,
            _: &'a mut (),
            frame: FrameCount,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>> {
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
            fn new(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Self {
                let input_1 = input_1::build(node);
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
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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

        fn give_output_tree(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
            OutputTree::new(output_1::build(node))
        }

        mod output_1 {
            use super::*;

            // types
            type SocketType = i64;
            pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

            // build socket
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                OutputSocket::new("output", Box::new(pickup), node)
            }

            // pick up from the output of node main process
            fn pickup(s: &NodeOutput) -> SocketType {
                *s
            }
        }
    }

    pub mod node_b {
        use crate::{
            node::{Node, NodeCommon},
            plugin::Plugin,
            socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
            types::{NodeName, SocketId},
            FrameCount,
        };
        use envelope::Envelope;
        use std::sync::{Arc, Weak};

        // Types of Node

        type NodeMemory = ();
        type NodeOutput = i64;

        // Node

        pub struct PluginB;

        #[async_trait::async_trait]
        impl Plugin for PluginB {
            fn name(&self) -> &'static str {
                "*1"
            }

            fn plugin_id(&self) -> crate::types::PluginId {
                crate::types::PluginId::from_string("9a2f7e5c-4946-ee07-b298-c92eca0ce1f0")
            }

            async fn build(&self) -> Arc<dyn NodeCommon> {
                Node::new(
                    "*2",
                    Inputs::new,
                    (),
                    Box::new(node_main_process),
                    give_output_tree,
                )
            }

            async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCommon>, &[u8]) {
                todo!()
            }
        }

        fn node_main_process<'a>(
            _: &'a NodeName,
            input: &'a Inputs,
            _: &'a mut (),
            frame: FrameCount,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>> {
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
            fn new(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Self {
                let input_1 = input_1::build(node);
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
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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

        fn give_output_tree(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
            OutputTree::new(output_1::build(node))
        }

        mod output_1 {
            use super::*;

            // types
            type SocketType = i64;
            pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

            // build socket
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                OutputSocket::new("output", Box::new(pickup), node)
            }

            // pick up from the output of node main process
            fn pickup(s: &NodeOutput) -> SocketType {
                *s
            }
        }
    }

    pub mod node_c {
        use crate::{
            node::{Node, NodeCommon},
            plugin::Plugin,
            socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
            types::{NodeName, SocketId},
            FrameCount,
        };
        use envelope::Envelope;
        use std::sync::{Arc, Weak};

        // Types of Node

        type NodeMemory = ();
        type NodeOutput = i64;

        // Node

        pub struct PluginC;

        #[async_trait::async_trait]
        impl Plugin for PluginC {
            fn name(&self) -> &'static str {
                "*3"
            }

            fn plugin_id(&self) -> crate::types::PluginId {
                crate::types::PluginId::from_string("a786a1c0-8103-2355-27d8-5962d459f450")
            }

            async fn build(&self) -> Arc<dyn NodeCommon> {
                Node::new(
                    "*3",
                    Inputs::new,
                    (),
                    Box::new(node_main_process),
                    give_output_tree,
                )
            }

            async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCommon>, &[u8]) {
                todo!()
            }
        }

        fn node_main_process<'a>(
            _: &'a NodeName,
            input: &'a Inputs,
            _: &'a mut (),
            frame: FrameCount,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>> {
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
            fn new(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Self {
                let input_1 = input_1::build(node);
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
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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

        fn give_output_tree(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
            OutputTree::new(output_1::build(node))
        }

        mod output_1 {
            use super::*;

            // types
            type SocketType = i64;
            pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

            // build socket
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                OutputSocket::new("output", Box::new(pickup), node)
            }

            // pick up from the output of node main process
            fn pickup(s: &NodeOutput) -> SocketType {
                *s
            }
        }
    }

    pub mod node_d {
        use crate::{
            node::{Node, NodeCommon},
            plugin::Plugin,
            socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
            types::{NodeName, SocketId},
            FrameCount,
        };
        use envelope::Envelope;
        use std::sync::{Arc, Weak};

        // Types of Node

        type NodeMemory = ();
        type NodeOutput = i64;

        // Node

        pub struct PluginD;

        #[async_trait::async_trait]
        impl Plugin for PluginD {
            fn name(&self) -> &'static str {
                "PAW"
            }

            fn plugin_id(&self) -> crate::types::PluginId {
                crate::types::PluginId::from_string("1ae64590-4520-8a70-f576-aa8b6cb08d18")
            }

            async fn build(&self) -> Arc<dyn NodeCommon> {
                Node::new(
                    "PAW",
                    Inputs::new,
                    (),
                    Box::new(node_main_process),
                    give_output_tree,
                )
            }

            async fn build_from_binary(&self, _: &[u8]) -> (Box<dyn NodeCommon>, &[u8]) {
                todo!()
            }
        }

        fn node_main_process<'a>(
            _: &'a NodeName,
            input: &'a Inputs,
            _: &'a mut (),
            frame: FrameCount,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>> {
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
            fn new(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Self {
                let input_1 = input_1::build(node);
                let input_2 = input_1::build(node);
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
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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

        fn give_output_tree(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> OutputTree {
            OutputTree::new(output_1::build(node))
        }

        mod output_1 {
            use super::*;

            // types
            type SocketType = i64;
            pub type Socket = OutputSocket<SocketType, Inputs, NodeMemory, NodeOutput>;

            // build socket
            pub fn build(node: &Weak<Node<Inputs, NodeMemory, NodeOutput>>) -> Arc<Socket> {
                OutputSocket::new("output", Box::new(pickup), node)
            }

            // pick up from the output of node main process
            fn pickup(s: &NodeOutput) -> SocketType {
                *s
            }
        }
    }
}
