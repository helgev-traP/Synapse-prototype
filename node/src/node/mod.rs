pub mod types;
pub mod node;
pub mod socket;
pub mod channel;
pub mod err;
pub mod field;
pub mod framework;

// System architectures

pub const BITS: usize = usize::BITS as usize;
pub const BYTES: usize = BITS / 8;
pub type FrameCount = i64;
