mod input;
pub use input::*;
mod output;
pub use output::*;

use crate::err::NodeConnectError;

/// connect two sockets

pub(crate) async fn connect(
    upstream: WeakOutputSocket,
    downstream: WeakInputSocket,
) -> Result<(), NodeConnectError> {
    let arc_upstream = upstream.weak().upgrade().unwrap();
    let arc_downstream = downstream.weak().upgrade().unwrap();

    // check socket type
    if arc_upstream.type_id() != arc_downstream.type_id() {
        return Err(NodeConnectError::TypeRejected);
    }

    // ensure the downstream socket is not connected
    let _ = arc_downstream.disconnect();

    arc_upstream.connect(downstream.weak());
    arc_downstream.connect(upstream.weak());

    Ok(())
}

pub(crate) async fn conservative_connect(
    upstream: WeakOutputSocket,
    downstream: WeakInputSocket,
) -> Result<(), NodeConnectError> {
    let arc_upstream = upstream.weak().upgrade().unwrap();
    let arc_downstream = downstream.weak().upgrade().unwrap();

    // check socket type
    if arc_upstream.type_id() != arc_downstream.type_id() {
        return Err(NodeConnectError::TypeRejected);
    }

    // check the downstream socket is not connected
    if arc_downstream.get_upstream_socket_id().await.is_some() {
        return Err(NodeConnectError::InputNotEmpty);
    }

    arc_upstream.connect(downstream.weak());
    arc_downstream.connect(upstream.weak());

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
