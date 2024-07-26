use super::types::SharedAny;

// --- Node connect and disconnect errors ---
#[derive(Debug, PartialEq)]
pub enum NodeConnectError {
    NodeIdNotFound,
    SocketIdNotFound,
    TypeRejected,
}

#[derive(Debug, PartialEq)]
pub enum NodeDisconnectError {
    NotConnected,
    NodeIdNotFound,
    SocketIdNotFound,
}

// --- Node connection check errors ---
#[derive(Debug, PartialEq)]
pub enum NodeConnectionCheckError {
    NodeIdNotFound,
    SocketIdNotFound,
    ChannelClosed,
    NotConnected,
}

// --- Node send order, response errors ---
#[derive(Debug, PartialEq)]
pub enum NodeSendOrderError {
}

#[derive(Debug, PartialEq)]
pub enum NodeSendResponseError {
    ChannelClosed,
    DownstreamNodeIdNotFound,
}

// --- Node update input default value errors ---
#[derive(Debug)]
pub enum UpdateInputDefaultError{
    NodeIdNotFound(SharedAny),
    SocketIdNotFound(SharedAny),
    TypeRejected(SharedAny),
}