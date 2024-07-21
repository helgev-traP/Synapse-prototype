use std::{
    any::TypeId,
    sync::{
        mpsc::{self, TryRecvError},
        Arc, RwLock,
    },
};

// channel

pub struct ChannelCom<TX, RX> {
    tx: mpsc::Sender<TX>,
    rx: mpsc::Receiver<RX>,
}

impl<T, S> ChannelCom<T, S> {
    pub fn new(tx: mpsc::Sender<T>, rx: mpsc::Receiver<S>) -> Self {
        ChannelCom { tx, rx }
    }
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
    Request { frame: i64 },
    Type,
}

pub enum NodeResponse {
    Response { frame: i64 },
    ProcessFaild,
    TypeId(TypeId),
    Shared(Arc<RwLock<dyn shared::Shared>>),
}

// shared memory

pub mod shared {
    use std::any::{Any, TypeId};

    pub trait Shared {
        fn type_id(&self) -> TypeId;
        fn value(&self) -> Box<dyn Any>;
    }

    impl<T: 'static> Shared for T {
        fn type_id(&self) -> TypeId {
            TypeId::of::<T>()
        }

        fn value(&self) -> Box<dyn Any> {
            todo!()
        }
    }

    pub fn is_same_id<T: 'static, U: 'static>(_t: T, _u: U) -> bool {
        TypeId::of::<T>() == TypeId::of::<U>()
    }

    pub fn cast_to<T: 'static + Copy>(any: &dyn Any) -> Option<T> {
        match any.downcast_ref::<T>() {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

pub struct Node {}

pub mod node_core {
    // ここの中はスレッドで生成実行される
    // メインスレッドでは触らない

    use super::*;
    use std::{any::Any, sync::mpsc::TryRecvError};

    // socket

    pub enum InputSocket {
        Socket {
            // input
            shared: Option<Arc<RwLock<dyn shared::Shared>>>,
            com: Option<ChannelCom<NodeOrder, NodeResponse>>,

            // default value
            default: Arc<RwLock<dyn shared::Shared>>,

            socket_name: String,
        },
        Group {
            sockets: Vec<InputSocket>,
            group_name: String,
        },
    }

    pub enum OutputSocket {
        Socket {
            shared: Arc<RwLock<dyn shared::Shared>>,
            com: Vec<ChannelCom<NodeResponse, NodeOrder>>,
            socket_name: String,
        },
        Group {
            sockets: Vec<OutputSocket>,
            group_name: String,
        },
    }

    // node

    enum ProcessStatus {
        WaitRequest,
        Process,
        Response,
        Shutdown,
        AfterShutdown,
    }

    pub struct NodeCore {
        // identify
        node_id: String,
        node_name: String,
        // com
        com_backend: ChannelCom<NodeToBackend, BackendToNode>,
        com_frontend: ChannelCom<NodeToFrontend, FrontendToNode>,
        // socket
        input_sockets: InputSocket,
        output_sockets: OutputSocket,
        // memory
        frame: i64,
        // main graphic process
        // todo ここのクロージャの型
        main_process: Box<dyn Fn() -> Result<(), ProcessStatus>>,
    }

    impl NodeCore {
        fn get_node_id(&self) -> &String {
            &self.node_id
        }

        fn get_node_name(&self) -> &String {
            &self.node_name
        }

        /// set main graphic process
        pub fn set_main_process(&mut self, f: Box<dyn Fn() -> Result<(), ProcessStatus>>) {
            self.main_process = f;
        }

        // com
        fn send_backend(&self, msg: NodeToBackend) {
            self.com_backend.tx.send(msg);
        }

        /// search output_sockets recursively, return the first found request.
        fn try_recv_request_recursive(&self, output_socket: &OutputSocket) -> Option<NodeOrder> {
            match output_socket {
                OutputSocket::Socket {
                    shared: _,
                    com,
                    socket_name: _,
                } => {
                    for ch in com {
                        if let Ok(order) = ch.rx.try_recv() {
                            return Some(order);
                        }
                    }
                    return None;
                }
                OutputSocket::Group {
                    sockets,
                    group_name,
                } => {
                    for i in sockets {
                        if let Some(order) = self.try_recv_request_recursive(i) {
                            return Some(order);
                        }
                    }
                    return None;
                }
            }
        }

        /// send request recursive
        fn send_request_recursive(&self, input_socket: &InputSocket) {
            // send
            match input_socket {
                InputSocket::Socket {
                    shared: _,
                    com,
                    default: _,
                    socket_name: _,
                } => {
                    if let Some(ch) = com {
                        if let Err(_) = ch.tx.send(NodeOrder::Request { frame: self.frame }) {
                            // todo err チャンネルの通信先がDropしていた場合
                            todo!();
                        }
                    }
                }

                InputSocket::Group {
                    sockets,
                    group_name: _,
                } => {
                    for i in sockets {
                        self.send_request_recursive(i);
                    }
                }
            }
        }

        /// wait response recursive
        fn wait_response_recursive(&self, input_socket: &InputSocket) {
            match input_socket {
                InputSocket::Socket {
                    shared: _,
                    com,
                    default: _,
                    socket_name: _,
                } => {
                    if let Some(ch) = com {
                        // do nothing if com is None
                        match ch.rx.recv() {
                            Ok(response) => match response {
                                NodeResponse::Response { frame } => {
                                    if frame != self.frame {
                                        todo!();
                                    }
                                }
                                NodeResponse::TypeId(_) => todo!(),
                                NodeResponse::Shared(_) => todo!(),
                            },
                            Err(err) => todo!(), // todo err チャンネルの通信先がDropしていた場合
                        }
                    }
                }

                InputSocket::Group {
                    sockets,
                    group_name: _,
                } => {
                    for i in sockets {
                        self.wait_response_recursive(i);
                    }
                }
            }
        }

        /// send responcse recursive
        fn send_response_recursive(
            &self,
            output_socket: &OutputSocket,
            response_message: NodeResponse,
        ) {
            match output_socket {
                OutputSocket::Socket {
                    shared: _,
                    com,
                    socket_name: _,
                } => {
                    for ch in com {
                        if let Err(order) = ch.tx.send(NodeResponse::Response { frame: self.frame })
                        {
                            todo!(); // todo チャンネル通信先がドロップ
                        }
                    }
                }
                OutputSocket::Group {
                    sockets,
                    group_name,
                } => {
                    for i in sockets {
                        self.send_response_recursive(i, response_message);
                    }
                }
            }
        }

        /// access to shared memory.
        ///
        /// if index is wrong, return None(too less index) or Some<Box<dyn Any>>(too much index).
        pub fn access_input_shared_memory(
            &self,
            index: Vec<i64>,
        ) -> Result<&Arc<RwLock<dyn shared::Shared>>, Option<&Arc<RwLock<dyn shared::Shared>>>>
        {
            let mut seek_point = &self.input_sockets;

            for i in index {
                match seek_point {
                    InputSocket::Socket {
                        shared,
                        com: _,
                        default,
                        socket_name: _,
                    } => match shared {
                        Some(x) => return Err(Some(x)),
                        None => return Err(Some(default)),
                    },
                    InputSocket::Group {
                        sockets,
                        group_name: _,
                    } => {
                        seek_point = &sockets[i as usize];
                    }
                }
            }

            match seek_point {
                InputSocket::Socket {
                    shared,
                    com: _,
                    default,
                    socket_name: _,
                } => match shared {
                    Some(shared) => return Ok(shared),
                    None => return Ok(default),
                },
                InputSocket::Group {
                    sockets: _,
                    group_name: _,
                } => Err(None),
            }
        }

        pub fn access_output_shared_memory(
            &mut self,
            index: Vec<i64>,
        ) -> Result<
            &mut Arc<RwLock<dyn shared::Shared>>,
            Option<&mut Arc<RwLock<dyn shared::Shared>>>,
        > {
            let mut seek_point = &mut self.input_sockets;

            for i in index {
                match seek_point {
                    InputSocket::Socket {
                        shared,
                        com: _,
                        ref mut default,
                        socket_name: _,
                    } => match shared {
                        Some(ref mut x) => return Err(Some(x)),
                        None => return Err(Some(default)),
                    },
                    InputSocket::Group {
                        sockets,
                        group_name: _,
                    } => {
                        seek_point = &mut sockets[i as usize];
                    }
                }
            }

            match seek_point {
                InputSocket::Socket {
                    ref mut shared,
                    com: _,
                    ref mut default,
                    socket_name: _,
                } => match shared {
                    Some(ref mut shared) => return Ok(shared),
                    None => return Ok(default),
                },
                InputSocket::Group {
                    sockets: _,
                    group_name: _,
                } => return Err(None),
            }
        }

        // # fns for main loop
        // ノードの状態遷移はこっち側で変更する

        fn wait_request(&mut self) -> ProcessStatus {
            loop {
                if let Ok(message) = self.com_backend.rx.try_recv() {
                    match message {
                        BackendToNode::Shutdown => {
                            return ProcessStatus::Shutdown;
                        }
                    }
                }
                if let Some(order) = self.try_recv_request_recursive(&self.output_sockets) {
                    match order {
                        NodeOrder::Request { frame } => {
                            self.frame = frame;
                            return ProcessStatus::Process;
                        }
                        NodeOrder::Type => todo!(),
                    }
                }
            }
        }

        fn pre_process(&mut self) -> ProcessStatus {
            self.send_request_recursive(&self.input_sockets);
            self.wait_response_recursive(&self.input_sockets);
            ProcessStatus::Process
        }

        /// # note
        /// 基本的に、
        /// ```
        /// self.pre_process();
        /// match self.process() {
        ///     Ok(_) => todo!(),
        ///     Err(_) => todo!(),
        /// }
        /// self.post_process();
        /// ```
        /// の順番で使う。
        fn process(&mut self) -> ProcessStatus {
            match (self.main_process)() {
                Ok(_) => ProcessStatus::Process,
                Err(err) => todo!(),
            }
        }

        fn post_process(&mut self) -> ProcessStatus {
            // insert something if necessary.
            ProcessStatus::Response
        }

        fn response(&mut self, response_message: NodeResponse) -> ProcessStatus {
            self.send_response_recursive(&self.output_sockets, response_message);
            ProcessStatus::WaitRequest
        }

        fn shutdown(&self) -> ProcessStatus {
            // todo
            ProcessStatus::AfterShutdown
        }

        /// start main loop
        pub fn main_loop(&mut self) {
            let mut status = ProcessStatus::WaitRequest;
            loop {
                match self.status {
                    ProcessStatus::WaitRequest => {
                        // todo ループをここで回すか wait_request()で回すか考える -> 関数の方で回す。パターンマッチの手間を省ける
                        status = self.wait_request();
                    }
                    ProcessStatus::Process => {
                        status = self.pre_process();
                        status = self.process();
                        if let ProcessStatus::Process = status {
                        } else {
                            continue;
                        }
                        status = self.post_process();
                    }
                    ProcessStatus::Response => {
                        status = self.response(NodeResponse::Response { frame: self.frame });
                    }
                    ProcessStatus::Shutdown => {
                        status = self.shutdown();
                    }
                    ProcessStatus::AfterShutdown => break, // ループ脱出
                }
            }
        }
    }
}
