// Errors of NodeCore
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

#[derive(Debug, PartialEq)]
pub enum NodeConnectionCheckError {
    NodeIdNotFound,
    SocketIdNotFound,
    ChannelClosed,
    NotConnected,
}

#[derive(Debug, PartialEq)]
pub enum NodeSendOrderError {
}

#[derive(Debug, PartialEq)]
pub enum NodeSendResponseError {
    ChannelClosed,
    DownstreamNodeIdNotFound,
}