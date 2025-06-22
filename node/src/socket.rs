pub mod input;
pub use input::*;
pub mod output;
pub use output::*;

use crate::err::NodeConnectError;

/// connect two sockets
pub(crate) async fn connect(
    upstream: OutputSocketCapsule,
    downstream: InputSocketCapsule,
) -> Result<(), NodeConnectError> {
    // check socket type
    if upstream.socket_type() != downstream.socket_type() {
        return Err(NodeConnectError::TypeRejected);
    }

    // ensure the downstream socket is not connected
    let _ = downstream.disconnect();

    upstream.connect(downstream.clone()).await;
    downstream.connect(upstream).await;

    Ok(())
}

pub(crate) async fn conservative_connect(
    upstream: OutputSocketCapsule,
    downstream: InputSocketCapsule,
) -> Result<(), NodeConnectError> {
    // check socket type
    if upstream.socket_type() != downstream.socket_type() {
        return Err(NodeConnectError::TypeRejected);
    }

    // check the downstream socket is not connected
    if downstream.upstream_socket_id().await.is_some() {
        return Err(NodeConnectError::InputNotEmpty);
    }

    upstream.connect(downstream.clone()).await;
    downstream.connect(upstream).await;

    Ok(())
}

/// --- Tests ---
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn input_minimum() {
        // todo
    }

    #[tokio::test]
    async fn input_memory() {
        // todo
    }

    #[tokio::test]
    async fn input_speed_benchmark() {
        // todo
    }

    #[tokio::test]
    async fn input_envelope() {
        // todo
    }
}
