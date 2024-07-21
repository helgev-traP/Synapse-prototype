pub mod types;
pub mod node;
pub mod node_field;
pub mod node_framework;

// System architectures

const BITS: usize = usize::BITS as usize;
const BYTES: usize = BITS / 8;
type FrameCount = i64;
