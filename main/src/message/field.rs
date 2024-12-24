use std::{any::TypeId, collections::HashSet};

use node::{
    types::{NodeId, PluginId, SharedAny, SocketId},
    FrameCount,
};

pub enum FieldReq {
    Id,
    Name,
    AllNodes(Vec<NodeId>),
    CacheDepth { node: NodeId },
}

pub enum FieldOp {
    // field operation
    AddNode(TypeId),
    RemoveNode(NodeId),
    Connect {
        upstream_node: NodeId,
        upstream_socket: SocketId,
        downstream_node: NodeId,
        downstream_socket: SocketId,
    },
    ConservativeConnect {
        upstream_node: NodeId,
        upstream_socket: SocketId,
        downstream_node: NodeId,
        downstream_socket: SocketId,
    },
    Disconnect {
        downstream_node: NodeId,
        downstream_socket: SocketId,
    },
    ConservativeDisconnect {
        upstream_node: NodeId,
        upstream_socket: SocketId,
        downstream_node: NodeId,
        downstream_socket: SocketId,
    },
    CheckConnection {
        upstream_node: NodeId,
        upstream_socket: SocketId,
        downstream_node: NodeId,
        downstream_socket: SocketId,
    },
    DisconnectAllFromOutput {
        node: NodeId,
        socket: SocketId,
    },
    DisconnectAll {
        node: NodeId,
    },
    UpdateInputDefault {
        node: NodeId,
        socket: SocketId,
        default: Box<SharedAny>,
    },
    CacheClear(NodeId),
    CacheSetDepth {
        node: NodeId,
        depth: usize,
    },
    CacheClearAll,
    CacheSetDepthAll(usize),
    Call {
        node: NodeId,
        frame: FrameCount,
    },
    Play {
        node: NodeId,
        frame: FrameCount,
    },
    Stop,
}
