use std::{any::{Any, TypeId}, cell::RefCell};

use super::id::SocketId;

// ## Shared
pub trait Shared: Send + Sync + 'static + Any {
    fn to_binary(&self) -> Vec<u8>;
    fn from_binary(binary: &[u8]) -> (Self, &[u8])
    where
        Self: Sized;
    fn type_id(&self) -> TypeId;
}

// ## SocketData

pub struct SocketData<T: Shared + Clone> {
    data: Option<Box<dyn Shared>>,
    default: Box<T>,
    id: SocketId,
    socket: RefCell<InputSocket>,
}

impl<T: Shared + Clone + 'static> SocketData<T> {
    pub fn new(data: T) -> Self {
        SocketData {
            data: None,
            default: Box::new(data),
            id: SocketId::new(),
            socket: todo!(),
        }
    }

    pub fn to_binary(&self) -> Vec<u8> {
        // default and id
        let mut bin = vec![];
        bin.extend_from_slice(&self.id.to_binary());
        bin.extend_from_slice(&self.default.to_binary());
        bin
    }

    pub fn from_binary(binary: &[u8]) -> (Self, &[u8]) {
        let (mut id, mut binary) = SocketId::from_binary(binary);
        let (mut default, mut binary) = T::from_binary(binary);
        (
            SocketData {
                data: None,
                default: Box::new(default),
                id,
                socket: todo!(),
            },
            binary,
        )
    }
}

// # Socket

pub struct InputSocket {
}

// todo 整理ができたらここのモジュールを一個上に -> 新しくモジュールを構築 ( -> Node-v3)
pub mod input {
/*
```
    use std::{
        any::{Any, TypeId},
        panic,
        sync::{
            mpsc::{RecvError, TryRecvError},
            Arc, RwLock,
        },
    };

    use super::{Channel, InputChannel, NodeOrder, NodeResponse, OutputChannel, Shared, SocketId};

    pub struct InputSocketOld {
        id: String,
        shared: Option<Arc<RwLock<dyn Shared>>>,
        data_default: Arc<RwLock<dyn Shared>>,
        data: Arc<RwLock<dyn Shared>>,
        channel: Option<Channel<NodeOrder, NodeResponse>>,
    }

    impl InputSocketOld {
        pub fn new(
            data: Arc<RwLock<dyn Shared>>,
            data_default: Arc<RwLock<dyn Shared>>,
            id: &String,
        ) -> Self {
            InputSocketOld {
                id: "socket-input-".to_string() + id,
                shared: None,
                data_default: data_default,
                data,
                channel: None,
            }
        }

        /// Connect a pair of socket

        pub fn connect(&mut self, com: InputChannel) -> Result<(), ()> {
            // 整理したい
            match com.rx.recv().unwrap() {
                NodeResponse::TypeId(type_id) => {
                    if type_id == (*self.data.read().unwrap()).type_id() {
                        com.tx.send(NodeOrder::TypeConformed);
                        self.channel = Some(com);
                        self.shared = match &self.channel {
                            Some(channel) => match channel.recv().unwrap() {
                                NodeResponse::Shared(shared) => Some(shared),
                                _ => panic!(""),
                            },
                            None => panic!(""),
                        };
                        Ok(())
                    } else {
                        com.tx.send(NodeOrder::TypeRejected);
                        Err(())
                    }
                }
                _ => panic!("node connection failed. type check failed."),
            }
        }

        /// disconnect socket

        pub fn disconnect(&mut self) {
            match &self.channel {
                Some(channel) => {
                    // Ignore SendError
                    let _ = channel.tx.send(NodeOrder::Disconnect);
                }
                None => panic!("channel not found error."),
            };
            self.channel = None;
        }

        /// send request

        pub fn send_request(&self, frame: i64) {
            if let Some(channel) = &self.channel {
                channel.tx.send(NodeOrder::Request { frame });
            }
        }

        /// send copy completed

        pub fn send_copy_completed(&self) {
            let _ = &self
                .channel
                .as_ref()
                .expect("")
                .send(NodeOrder::CopyCompleted)
                .unwrap();
        }

        /// Send any type of NodeOrder.
        ///
        /// **This method is not recommended to use.**

        pub fn send(&self, msg: NodeOrder) {
            match &self.channel {
                Some(channel) => {
                    channel.tx.send(msg);
                }
                None => todo!(),
            }
        }

        /// Wait response.

        pub fn wait_response(&mut self) {
            match &self.channel {
                Some(channel) => match channel.rx.recv().unwrap() {
                    NodeResponse::Response { frame } => {
                        let shared = self.shared.as_ref().expect("").read().unwrap();
                        let mut data = self.data.write().unwrap();
                        (*data).from_binary(&shared.to_binary());
                        self.send_copy_completed();
                    }
                    NodeResponse::ProcessFailed => {}
                    _ => panic!(""),
                },
                None => {
                    let data_default = self.data_default.read().unwrap();
                    let mut data = self.data.write().unwrap();
                    (*data).from_binary(&data_default.to_binary());
                }
            }
        }
    }

    /// OutputSocket for Nodes.
    ///
    /// ## methods:
    /// ### new
    /// - new
    /// ### connection
    /// - connect
    /// - disconnect
    /// ### communication
    /// - send_response
    /// - send_process_failed
    /// - send
    /// - try_recv_request
    /// - recv_request
    pub struct OutputSocketOld {
        id: SocketId,
        data: Arc<RwLock<dyn Shared>>, // todo トレイトをソケット用のものにするかも
        channel: Option<Channel<NodeResponse, NodeOrder>>,
    }

    impl OutputSocketOld {
        pub fn new(data: Arc<RwLock<dyn Shared>>, id: &String) -> Self {
            OutputSocketOld {
                id: "socket-output-".to_string() + id,
                data,
                channel: None,
            }
        }

        /// Connect a pair of socket

        pub fn connect(&mut self, com: OutputChannel) -> Result<(), ()> {
            com.send(NodeResponse::TypeId((*self.data.read().unwrap()).type_id()));
            match com.rx.recv().unwrap() {
                NodeOrder::TypeConformed => {
                    self.channel = Some(com);
                    match &self.channel {
                        Some(channel) => {
                            channel.tx.send(NodeResponse::Shared(self.data.clone()));
                            Ok(())
                        }
                        None => panic!(),
                    }
                }
                NodeOrder::TypeRejected => Err(()),
                _ => panic!("node connection failed. type conform massage error."),
            }
        }

        /// disconnect socket

        pub fn disconnect(&mut self) {
            match &self.channel {
                Some(channel) => {
                    // Ignore SendError
                    let _ = channel.tx.send(NodeResponse::Disconnect);
                }
                None => panic!("channel not found error."),
            };
            self.channel = None;
        }

        /// Send Response

        pub fn send_response(&self, frame: i64) {
            if let Some(channel) = &self.channel {
                channel.send(NodeResponse::Response { frame });
                match channel.recv().unwrap() {
                    NodeOrder::CopyCompleted => {}
                    _ => panic!(""),
                }
            }
        }

        /// Send ProcessFailed

        pub fn send_process_failed(&self) {
            if let Some(channel) = &self.channel {
                channel.send(NodeResponse::ProcessFailed);
            }
        }

        /// Send any type of NodeResponse.
        ///
        /// **This method is not recommended to use.**

        pub fn send(&self, msg: NodeResponse) {
            if let Some(channel) = &self.channel {
                channel.send(msg);
            }
        }

        /// try receive request

        pub fn try_recv_request(&self) -> Result<NodeOrder, TryRecvError> {
            if let Some(channel) = &self.channel {
                channel.try_recv()
            } else {
                Err(TryRecvError::Empty)
            }
        }

        /// receive request

        pub fn recv_request(&self) -> Result<NodeOrder, RecvError> {
            if let Some(channel) = &self.channel {
                channel.recv()
            } else {
                Err(RecvError)
            }
        }
    }
```
*/
}
