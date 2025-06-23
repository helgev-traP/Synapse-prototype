use std::any::TypeId;

use tokio::sync::mpsc::{self, error::SendError};
use uuid::Uuid;

use super::{
    err::{NodeConnectError, NodeDisconnectError, UpdateInputDefaultError},
    types::{NodeId, SharedAny, SocketId},
    FrameCount,
};

/// # Channel
/// A two way channel.
pub struct Channel<TX, RX> {
    tx: mpsc::Sender<TX>,
    rx: mpsc::Receiver<RX>,
}

impl<TX, RX> Channel<TX, RX> {
    pub fn new(tx: mpsc::Sender<TX>, rx: mpsc::Receiver<RX>) -> Self {
        Channel { tx, rx }
    }

    pub async fn send(&self, msg: TX) -> Result<(), mpsc::error::SendError<TX>> {
        self.tx.send(msg).await
    }

    pub fn try_recv(&mut self) -> Result<RX, mpsc::error::TryRecvError> {
        self.rx.try_recv()
    }

    pub async fn recv(&mut self) -> Option<RX> {
        self.rx.recv().await
    }

    pub async fn send_result(&mut self, msg: TX) -> Result<Option<RX>, SendError<TX>> {
        self.tx.send(msg).await?;
        match self.rx.recv().await {
            Some(r) => Ok(Some(r)),
            None => Ok(None),
        }
    }
}

/// Get a pair of `types::Channel`
/// # Example
/// ```
/// use node::channel::channel_pair;
///
/// #[tokio::main]
/// async fn main() {
///     let (mut ch_a, mut ch_b) = channel_pair::<i32, &str>(1);
///     ch_a.send(1).await.unwrap()    ;
///     assert_eq!(ch_b.recv().await.unwrap(), 1);
///
///     ch_b.send("hello").await.unwrap();
///     assert_eq!(ch_a.recv().await.unwrap(), "hello");
/// }
/// ```
pub fn channel_pair<T, S>(buffer: usize) -> (Channel<T, S>, Channel<S, T>) {
    let (tx1, rx2) = mpsc::channel::<T>(buffer);
    let (tx2, rx1) = mpsc::channel::<S>(buffer);
    (Channel::new(tx1, rx1), Channel::new(tx2, rx2))
}

/// # ResultChannel
/// A two way channel with result.
pub struct ResultChannel<TX, TxResult, RX, RxResult> {
    /// tx
    tx: mpsc::Sender<TX>,
    /// receive result of sended message
    tx_result: mpsc::Receiver<TxResult>,
    /// rx
    rx: mpsc::Receiver<RX>,
    /// send result of received message
    rx_result: mpsc::Sender<RxResult>,
}

impl<TX, TxResult, RX, RxResult> ResultChannel<TX, TxResult, RX, RxResult> {
    pub fn new(
        tx: mpsc::Sender<TX>,
        tx_result: mpsc::Receiver<TxResult>,
        rx: mpsc::Receiver<RX>,
        rx_result: mpsc::Sender<RxResult>,
    ) -> Self {
        ResultChannel {
            tx,
            tx_result,
            rx,
            rx_result,
        }
    }

    pub async fn send(&mut self, msg: TX) -> Result<Option<TxResult>, mpsc::error::SendError<TX>> {
        self.tx.send(msg).await?;
        Ok(self.tx_result.recv().await)
    }

    pub async fn try_recv<Fut>(
        &mut self,
        function: Box<dyn FnOnce(RX) -> Fut + Send>,
    ) -> Result<(), mpsc::error::TryRecvError>
    where
        Fut: std::future::Future<Output = RxResult>,
    {
        let rx = self.rx.try_recv()?;
        self.rx_result.send(function(rx).await).await.unwrap();
        Ok(())
    }

    pub async fn try_recv_generic<'a, Fut, Re>(
        &'a mut self,
        function: Box<dyn FnOnce(RX) -> Fut + Send + 'a>,
    ) -> Result<Re, mpsc::error::TryRecvError>
    where
        Fut: std::future::Future<Output = (RxResult, Re)> + 'a,
    {
        let rx = self.rx.try_recv()?;
        let (result, return_value) = function(rx).await;
        self.rx_result.send(result).await.unwrap();
        Ok(return_value)
    }

    pub async fn recv<Fut>(&mut self, function: Box<dyn FnOnce(RX) -> Fut + Send>) -> Option<()>
    where
        Fut: std::future::Future<Output = RxResult>,
    {
        let rx = self.rx.recv().await?;
        self.rx_result.send(function(rx).await).await.unwrap();
        Some(())
    }

    pub async fn recv_generic<'a, Fut, Re>(
        &mut self,
        function: Box<dyn FnOnce(RX) -> Fut + Send + 'a>,
    ) -> Option<Re>
    where
        Fut: std::future::Future<Output = (RxResult, Re)> + 'a,
    {
        let rx = self.rx.recv().await?;
        let (result, return_value) = function(rx).await;
        self.rx_result.send(result).await.unwrap();
        Some(return_value)
    }
}

pub type FrontChannel = ResultChannel<
    FrontToController,
    FrontToControllerResult,
    ControllerToFront,
    ControllerToFrontResult,
>;
pub type ControllerChannel = ResultChannel<
    ControllerToFront,
    ControllerToFrontResult,
    FrontToController,
    FrontToControllerResult,
>;

pub fn result_channel_pair<T, TResult, S, SResult>(
    buffer: usize,
) -> (
    ResultChannel<T, TResult, S, SResult>,
    ResultChannel<S, SResult, T, TResult>,
) {
    let (tx1, rx1) = mpsc::channel::<T>(buffer);
    let (tx2, rx2) = mpsc::channel::<S>(buffer);
    let (rx_result1, tx_result1) = mpsc::channel::<TResult>(buffer);
    let (rx_result2, tx_result2) = mpsc::channel::<SResult>(buffer);
    (
        ResultChannel::new(tx1, tx_result1, rx2, rx_result2),
        ResultChannel::new(tx2, tx_result2, rx1, rx_result1),
    )
}

// Controller <-> Frontend
pub enum FrontToController {
    /// # Shutdown
    /// End the main loop of the node controller.
    Shutdown,
    /// # NodeConnect
    NodeConnect {
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    },
    /// # NodeDisconnect
    /// Delete channel from input socket to disconnecting node, it will safely disconnected after the upstream output socket find channel was dropped.
    NodeDisconnect {
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    },
    /// # NodeDisconnectSafe
    /// Make node controller check if the connection is valid.
    NodeDisconnectSafe {
        upstream_node_id: NodeId,
        upstream_node_socket_id: SocketId,
        downstream_node_id: NodeId,
        downstream_node_socket_id: SocketId,
    },
    /// # UpdateInputDefaultValue
    /// Update the default value of the input socket.
    UpdateInputDefaultValue {
        node_id: NodeId,
        socket_id: SocketId,
        value: Box<SharedAny>,
    },
}

pub enum FrontToControllerResult {
    /// # ShutdownResult
    Shutdown(Result<(), ()>),
    /// # NodeConnectResult
    NodeConnect(Result<(), NodeConnectError>),
    /// # NodeDisconnectResult
    NodeDisconnect(Result<(), NodeDisconnectError>),
    /// # NodeDisconnectSafeResult
    NodeDisconnectSafe(Result<(), NodeDisconnectError>),
    /// # UpdateInputDefaultValueResult
    UpdateInputDefaultValue(Result<(), UpdateInputDefaultError>),
}

// todo Resultに移す
pub enum ControllerToFront {}

pub enum ControllerToFrontResult {}

// Node <-> Frontend
pub enum FrontToNode {}

pub enum NodeToFront {}
