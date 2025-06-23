use std::{any::Any, sync::Arc};

use super::node_core::NodeCoreCommon;

#[async_trait::async_trait]
pub trait Plugin: Send + Sync + Any {
    fn name(&self) -> &'static str;
    async fn build(&self) -> Arc<dyn NodeCoreCommon>;
    async fn build_from_binary(&self, binary: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]);
}

#[cfg(test)]
pub mod template {
    use super::Plugin;
    use crate::{
        node_core::{Node, NodeCoreCommon},
        socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
        types::{NodeName, SocketId},
        FrameCount,
    };
    use envelope::Envelope;
    use std::sync::{Arc, Weak};

    // Types of Node

    type NodeMemory = ();
    type NodeOutput = String;

    // Node

    pub struct Builder;

    #[async_trait::async_trait]
    impl Plugin for Builder {
        fn name(&self) -> &'static str {
            "Template"
        }

        async fn build(&self) -> Arc<dyn NodeCoreCommon> {
            Node::new(
                "Template",
                TemplateInput::new,
                (),
                Box::new(node_main_process),
                give_output_tree,
            )
        }

        // #[cfg(debug_assertions)]
        // async fn build_debug(
        //     &self,
        // ) -> (
        //     Arc<dyn NodeCoreCommon>,
        //     Vec<crate::types::SocketId>,
        //     Vec<crate::types::SocketId>,
        // ) {
        //     let node = NodeCore::new(
        //         "Template Debug",
        //         TemplateInput::new,
        //         (),
        //         Box::new(node_main_process),
        //         give_output_tree,
        //     );

        //     (node, vec![], vec![])
        // }

        async fn build_from_binary(&self, binary: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
            todo!()
        }
    }

    fn node_main_process<'a>(
        name: &'a NodeName,
        input: &'a TemplateInput,
        _: &'a mut NodeMemory,
        frame: FrameCount,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>> {
        Box::pin(async move {
            let input = input.input_1.get(frame).await;

            input + " via " + name
        })
    }

    // Input

    struct TemplateInput {
        input_1: Arc<input_1::Socket>,
    }

    #[async_trait::async_trait]
    impl InputGroup for TemplateInput {
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

    impl TemplateInput {
        fn new(node: &Weak<Node<TemplateInput, NodeMemory, NodeOutput>>) -> Self {
            Self {
                input_1: input_1::build(node),
            }
        }
    }

    // Input Sockets

    mod input_1 {
        use super::*;

        // types
        type Default = i64;
        type Memory = ();
        type SocketType = String;
        pub type Socket =
            InputSocket<Default, Memory, SocketType, TemplateInput, NodeMemory, NodeOutput>;

        // build socket
        pub fn build(node: &Weak<Node<TemplateInput, NodeMemory, NodeOutput>>) -> Arc<Socket> {
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
            _: &mut Memory,
            _: FrameCount,
        ) -> SocketType {
            default.unwrap().to_string()
        }
    }

    // Output

    fn give_output_tree(node: &Weak<Node<TemplateInput, NodeMemory, NodeOutput>>) -> OutputTree {
        OutputTree::new(output_1::build(node))
    }

    mod output_1 {
        use super::*;

        // types
        type SocketType = String;
        pub type Socket = OutputSocket<SocketType, TemplateInput, NodeMemory, NodeOutput>;

        // build socket
        pub fn build(node: &Weak<Node<TemplateInput, NodeMemory, NodeOutput>>) -> Arc<Socket> {
            OutputSocket::new("output", Box::new(pickup), node)
        }

        // pick up from the output of node main process
        fn pickup(s: &NodeOutput) -> SocketType {
            s.clone()
        }
    }
}
