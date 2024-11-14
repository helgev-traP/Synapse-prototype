use super::node_core::NodeCoreCommon;

pub trait NodeFramework {
    fn new() -> Box<dyn NodeCoreCommon>;
    fn build_from_binary(binary: &[u8]) -> (Box<dyn NodeCoreCommon>, &[u8]);
}
