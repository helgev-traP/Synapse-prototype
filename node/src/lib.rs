pub mod channel;
pub mod err;
pub mod node_controller;
pub mod node;
pub mod plugin;
pub mod plugin_manager;
pub mod socket;
pub mod types;

// System architectures

/// The architecture of the system.
pub const WORD_SIZE: usize = usize::BITS as usize;
/// How many bytes are in a word.
pub const BYTES_IN_WORD: usize = WORD_SIZE / 8;
pub type FrameCount = i64;
