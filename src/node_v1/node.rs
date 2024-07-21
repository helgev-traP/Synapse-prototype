pub struct Node {
    // identify
    node_id: Arc<Mutex<String>>,
    node_name: Arc<Mutex<String>>,
}

impl Node {}

pub mod node_core {
    // ここの中はスレッドで生成実行される
    // メインスレッドでは触らない

    use super::super::types::*;
    use std::{
        any::Any,
        ops::Mul,
        sync::{mpsc::TryRecvError, Arc, Mutex, RwLock},
    };

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

    enum RequestResult {
        //
        Request,
        DeleteCache,
        Shutdown,
    }

    pub struct NodeCore {
        // identify
        node_id: Arc<Mutex<String>>,
        node_name: Arc<Mutex<String>>,
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
        main_process: Box<dyn Fn() -> Result<(), ()>>,
        // 状態
        node_state_variable: dyn Any,
    }

    impl NodeCore {
        fn get_node_id(&self) -> &Arc<Mutex<String>> {
            &self.node_id
        }

        fn get_node_name(&self) -> &Arc<Mutex<String>> {
            &self.node_name
        }

        /// set main graphic process
        pub fn set_main_process(&mut self, f: Box<dyn Fn() -> Result<(), ()>>) {
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
                                NodeResponse::ProcessFailed => todo!(),
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
                        self.send_response_recursive(i, response_message.clone());
                    }
                }
            }
        }

        /// delete all cache
        fn delete_cache(&self) {
            todo!()
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

        // fns for main loop
        // ノードの状態遷移はこっち側で変更する

        fn wait_request(&mut self) -> RequestResult {
            loop {
                if let Ok(message) = self.com_backend.rx.try_recv() {
                    match message {
                        BackendToNode::Shutdown => {
                            return RequestResult::Shutdown;
                        }
                    }
                }
                if let Some(order) = self.try_recv_request_recursive(&self.output_sockets) {
                    match order {
                        NodeOrder::Request { frame } => {
                            self.frame = frame;
                            return RequestResult::Request;
                        }
                        NodeOrder::Type => todo!(),
                    }
                }
            }
        }

        fn send_request_and_wait(&mut self) {
            self.send_request_recursive(&self.input_sockets);
            self.wait_response_recursive(&self.input_sockets);
        }

        fn pre_process(&self) -> () {
            // insert something if necessary.
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
        fn main_process(&mut self) -> Result<(), ()> {
            // todo Errの内容をどうにかする
            match (self.main_process)() {
                Ok(_) => Ok(()),
                Err(err) => todo!(),
            }
        }

        fn post_process(&mut self) -> () {
            // insert something if necessary.
        }

        fn send_response(&mut self, response_message: NodeResponse) {
            self.send_response_recursive(&self.output_sockets, response_message);
        }

        fn shutdown(&self) {
            // todo
        }

        /// # Start main loop
        ///
        /// functions to be used:
        /// - wait_request
        /// - send_request_and_wait
        /// - pre_process
        /// - main_process
        /// - post_process
        /// - send_response
        pub fn main(&mut self) {
            'main_loop: loop {
                match self.wait_request() {
                    // delete cache
                    RequestResult::DeleteCache => {
                        self.delete_cache();
                        continue 'main_loop;
                    }
                    // shutdown
                    RequestResult::Shutdown => {
                        self.shutdown();
                        break 'main_loop;
                    }
                    // receive request. then process and response result to higher node.
                    RequestResult::Request => {
                        // process
                        self.send_request_and_wait();
                        self.pre_process();
                        match self.main_process() {
                            Err(_) => {
                                todo!();
                                self.send_response(NodeResponse::ProcessFailed);
                                continue 'main_loop;
                            }
                            Ok(_) => {
                                self.post_process();
                                self.send_response(NodeResponse::Response { frame: self.frame });
                            }
                        }
                    }
                }
            }
        }
    }
}
