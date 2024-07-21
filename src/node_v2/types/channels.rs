use std::{any::TypeId, sync::{mpsc::{self, RecvError, SendError, TryRecvError}, Arc, RwLock}};



pub struct Channel<TX, RX> {
    tx: mpsc::Sender<TX>,
    rx: mpsc::Receiver<RX>,
}

impl<TX, RX> Channel<TX, RX> {
    pub fn new(tx: mpsc::Sender<TX>, rx: mpsc::Receiver<RX>) -> Self {
        Channel { tx, rx }
    }

    pub fn send(&self, msg: TX) -> Result<(), SendError<TX>> {
        self.tx.send(msg)
    }

    pub fn try_recv(&self) -> Result<RX, TryRecvError> {
        self.rx.try_recv()
    }

    pub fn recv(&self) -> Result<RX, RecvError> {
        self.rx.recv()
    }
}

pub type InputChannel = Channel<NodeOrder, NodeResponse>;
pub type OutputChannel = Channel<NodeResponse, NodeOrder>;

/// Get a pair of `types::Channel`
/// # Example
/// ```
/// use node_system::node_forest::types::channel_pair;
/// let (ch_a, ch_b) = channel_pair::<i32, &str>();
/// ch_a.send(1);
/// assert_eq!(ch_b.recv().unwrap(), 1);
///
/// ch_b.send("hello");
/// assert_eq!(ch_a.recv().unwrap(), "hello");
/// ```
pub fn channel_pair<T, S>() -> (Channel<T, S>, Channel<S, T>) {
    let (tx1, rx2) = mpsc::channel::<T>();
    let (tx2, rx1) = mpsc::channel::<S>();
    (Channel::new(tx1, rx1), Channel::new(tx2, rx2))
}

pub enum BackendToNode {
    Shutdown,
}

pub enum NodeToBackend {
    Error,
}

pub enum FrontendToNode {}

pub enum NodeToFrontend {}

pub enum NodeOrder {
    Request {
        frame: i64,
    },
    // Copy Completed. 共有メモリでデータを渡すときにのみ使う
    CopyCompleted,
    TypeConformed,
    TypeRejected,
    /// disconnect message.
    ///
    /// Handle NodeOrder::Disconnect as much as possible.
    Disconnect,
}

#[derive(Clone)]
pub enum NodeResponse {
    Response {
        frame: i64,
    },
    ProcessFailed,
    TypeId(TypeId),
    Shared(todo!()),
    /// disconnect message.
    ///
    /// Handle NodeOrder::Disconnect as much as possible.
    Disconnect,
}