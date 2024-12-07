use std::sync::Arc;

use super::node_core::NodeCoreCommon;

#[async_trait::async_trait]
pub trait NodeFramework {
    async fn new() -> Arc<dyn NodeCoreCommon>;
    #[cfg(debug_assertions)]
    async fn new_debug() -> (
        Arc<dyn NodeCoreCommon>,
        Vec<crate::types::SocketId>,
        Vec<crate::types::SocketId>,
    );
    async fn build_from_binary(binary: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]);
}

#[cfg(test)]
pub mod template {
    use super::NodeFramework;
    use crate::{
        node_core::{NodeCore, NodeCoreCommon},
        socket::{InputGroup, InnerInputSocket, InputTrait, OutputSocket, OutputTree},
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
    impl NodeFramework for Builder {
        async fn new() -> Arc<dyn NodeCoreCommon> {
            let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

            let input = TemplateInput::new(node.clone());
            let output = give_output_tree(node.clone());

            node.set_input(input);
            node.set_output(output);

            node
        }

        #[cfg(debug_assertions)]
        async fn new_debug() -> (
            Arc<dyn NodeCoreCommon>,
            Vec<crate::types::SocketId>,
            Vec<crate::types::SocketId>,
        ) {
            let node = Arc::new(NodeCore::new("Template", (), Box::new(node_main_process)));

            let input = TemplateInput::new(node.clone());
            let output = give_output_tree(node.clone());

            node.set_input(input);
            node.set_output(output);

            (node, vec![], vec![])
        }

        async fn build_from_binary(binary: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
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
        async fn get_socket(&self, id: SocketId) -> Option<Weak<dyn InputTrait>> {
            if id == self.input_1.get_id() {
                Some(Arc::downgrade(&self.input_1) as Weak<dyn InputTrait>)
            } else {
                None
            }
        }
    }

    impl TemplateInput {
        fn new(node: Arc<NodeCore<TemplateInput, NodeMemory, NodeOutput>>) -> Self {
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
        type SocketType = String;
        pub type Socket =
            InnerInputSocket<Default, Memory, SocketType, TemplateInput, NodeMemory, NodeOutput>;

        // build socket
        pub fn build(node: Arc<NodeCore<TemplateInput, NodeMemory, NodeOutput>>) -> Socket {
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
            _: &mut Memory,
            _: FrameCount,
        ) -> SocketType {
            default.unwrap().to_string()
        }
    }

    // Output

    fn give_output_tree(node: Arc<NodeCore<TemplateInput, NodeMemory, NodeOutput>>) -> OutputTree {
        OutputTree::Socket(Arc::new(output_1::build(node)))
    }

    mod output_1 {
        use super::*;

        // types
        type SocketType = String;
        pub type Socket = OutputSocket<SocketType, TemplateInput, NodeMemory, NodeOutput>;

        // build socket
        pub fn build(node: Arc<NodeCore<TemplateInput, NodeMemory, NodeOutput>>) -> Socket {
            OutputSocket::new("output", Box::new(pickup), node)
        }

        // pick up from the output of node main process
        fn pickup(s: &NodeOutput) -> SocketType {
            s.clone()
        }
    }
}
