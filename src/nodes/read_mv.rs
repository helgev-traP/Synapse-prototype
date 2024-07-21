use crate::node_forest::{framework::NodeFramework, node::NodeCoreCommon};

struct ReadMv {}

impl NodeFramework for ReadMv {
    fn new() -> Box<dyn NodeCoreCommon> {
        todo!()
    }

    fn build_from_binary(binary: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]) {
        todo!()
    }
}
