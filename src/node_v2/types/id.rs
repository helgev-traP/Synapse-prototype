use uuid::Uuid;

use super::super::BYTES;

/// NodeId
/// - NodeId is a unique identifier generated by Uuid for each node.
/// # Example
/// ```
/// use node_system::node_forest::types::NodeId;
/// let id = NodeId::new();
/// let id_str = id.to_string();
/// assert_eq!(id, NodeId::from_string(&id_str));
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(Uuid);

impl NodeId {
    pub fn new() -> Self {
        NodeId(Uuid::new_v4())
    }
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
    pub fn from_string(id: &str) -> Self {
        NodeId(Uuid::parse_str(id).unwrap())
    }

    pub fn to_binary(&self) -> Vec<u8> {
        let id_str = self.to_string();
        let len = id_str.len();
        let mut bin = vec![0; len + BYTES];
        bin[0..BYTES].copy_from_slice(&len.to_be_bytes());
        bin[BYTES..].copy_from_slice(id_str.as_bytes());
        bin
    }

    pub fn from_binary(binary: &[u8]) -> (Self, &[u8]) {
        let len = usize::from_be_bytes(binary[0..BYTES].try_into().unwrap());
        let id_str = String::from_utf8(binary[BYTES..BYTES + len].to_vec()).unwrap();
        (NodeId::from_string(&id_str), &binary[BYTES + len..])
    }
}

/// SocketId
/// - SocketId is a unique identifier generated by Uuid for each socket.
/// # Example
/// ```
/// use node_system::node_forest::types::SocketId;
/// let id = SocketId::new();
/// let id_str = id.to_string();
/// assert_eq!(id, SocketId::from_string(&id_str));
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SocketId(Uuid);

impl SocketId {
    pub fn new() -> Self {
        SocketId(Uuid::new_v4())
    }
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
    pub fn from_string(id: &String) -> Self {
        SocketId(Uuid::parse_str(id).unwrap())
    }

    pub fn to_binary(&self) -> Vec<u8> {
        let id_str = self.to_string();
        let len = id_str.len();
        let mut bin = vec![0; len + BYTES];
        bin[0..BYTES].copy_from_slice(&len.to_be_bytes());
        bin[BYTES..].copy_from_slice(id_str.as_bytes());
        bin
    }

    pub fn from_binary(binary: &[u8]) -> (Self, &[u8]) {
        let len = usize::from_be_bytes(binary[0..BYTES].try_into().unwrap());
        let id_str = String::from_utf8(binary[BYTES..BYTES + len].to_vec()).unwrap();
        (SocketId::from_string(&id_str), &binary[BYTES + len..])
    }
}