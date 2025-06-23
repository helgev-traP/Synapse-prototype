use envelope::Envelope;
use node::plugin::Plugin;
use node::{
    node_core::{Node, NodeCoreCommon},
    socket::{InputGroup, InputSocket, InputSocketCapsule, OutputSocket, OutputTree},
    types::{NodeName, SocketId},
    FrameCount,
};
use std::sync::{Arc, Weak};

// Types of Node

type NodeMemory = ();
type NodeOutput = i64;

// Node

pub struct CurrentFrameCount;

#[async_trait::async_trait]
impl Plugin for CurrentFrameCount {
    fn name(&self) -> &'static str {
        "current frame count"
    }

    async fn build(&self) -> Arc<dyn NodeCoreCommon> {
        Node::new(
            "current frame count",
            TemplateInput::new,
            (),
            Box::new(node_main_process),
            give_output_tree,
        )
    }

    async fn build_from_binary(&self, binary: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
        todo!()
    }
}

fn node_main_process<'a>(
    _: &'a NodeName,
    input: &'a TemplateInput,
    _: &'a mut NodeMemory,
    frame: FrameCount,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeOutput> + Send + 'a>> {
    Box::pin(async move {
        let input = input.input_1.get(frame).await;

        input
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
    type Default = ();
    type Memory = ();
    type SocketType = i64;
    pub type Socket =
        InputSocket<Default, Memory, SocketType, TemplateInput, NodeMemory, NodeOutput>;

    // build socket
    pub fn build(node: &Weak<Node<TemplateInput, NodeMemory, NodeOutput>>) -> Arc<Socket> {
        InputSocket::new(
            "input",
            node,
            "",
            None,
            "frame count",
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
        _: &mut Memory,
        frame: FrameCount,
    ) -> SocketType {
        envelope.unwrap().value(frame as f64) as i64
    }
}

// Output

fn give_output_tree(node: &Weak<Node<TemplateInput, NodeMemory, NodeOutput>>) -> OutputTree {
    OutputTree::new(output_1::build(node))
}

mod output_1 {
    use super::*;

    // types
    type SocketType = i64;
    pub type Socket = OutputSocket<SocketType, TemplateInput, NodeMemory, NodeOutput>;

    // build socket
    pub fn build(node: &Weak<Node<TemplateInput, NodeMemory, NodeOutput>>) -> Arc<Socket> {
        OutputSocket::new("output", Box::new(pickup), node)
    }

    // pick up from the output of node main process
    fn pickup(s: &NodeOutput) -> SocketType {
        *s
    }
}
