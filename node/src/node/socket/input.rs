use core::panic;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::Mutex;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::node::{
    channel::{InputChannel, NodeOrder, NodeResponse, OutputChannel},
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, NodeSendResponseError,
        UpdateInputDefaultError,
    },
    types::{Envelope, NodeId, SharedAny, SocketId, TryRecvResult},
    FrameCount,
};

/// # ReadFn
// todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
pub trait ReadFn<DATA, MEMORY, OUTPUT>: Send + Sync
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    fn call(
        &self,
        default: &DATA,
        memory: &mut MEMORY,
        envelope: &Envelope,
        frame: FrameCount,
    ) -> OUTPUT;
}

impl<DATA, MEMORY, OUTPUT, F> ReadFn<DATA, MEMORY, OUTPUT> for F
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
    F: Fn(&DATA, &mut MEMORY, &Envelope, FrameCount) -> OUTPUT + Send + Sync,
{
    fn call(
        &self,
        default: &DATA,
        memory: &mut MEMORY,
        envelope: &Envelope,
        frame: FrameCount,
    ) -> OUTPUT {
        self(default, memory, envelope, frame)
    }
}

/// # FrameFn
// todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
pub trait FrameFn: Send + Sync {
    fn call(&self, frame: FrameCount, envelope: &Envelope) -> FrameCount;
}

impl<F> FrameFn for F
where
    F: Fn(FrameCount, &Envelope) -> FrameCount + Send + Sync,
{
    fn call(&self, frame: FrameCount, envelope: &Envelope) -> FrameCount {
        self(frame, envelope)
    }
}

/// # InputSocketChannel
// private
struct InputSocketChannel {
    pub channel: InputChannel,
    pub upstream_socket_id: SocketId,
}

/// # InputSocket
// private
struct InputSocket<DATA, MEMORY, OUTPUT>
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    // channel
    channel: Option<InputSocketChannel>,
    // closure
    read_fn: Box<dyn ReadFn<DATA, MEMORY, OUTPUT>>,
    memory: MEMORY,
    // frame
    frame_fn: Box<dyn FrameFn>,
}

impl<DATA, MEMORY, OUTPUT> InputSocket<DATA, MEMORY, OUTPUT>
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    async fn transcript(
        &mut self,
        default: &Arc<RwLock<DATA>>,
        envelope_data: &Envelope,
        envelope_frame: &Envelope,
        frame: FrameCount,
    ) -> SharedAny {
        match self.channel {
            Some(ref mut channel) => {
                channel
                    .channel
                    .send(NodeOrder::Request {
                        frame: self.frame_fn.call(frame, envelope_frame),
                    })
                    .await
                    .unwrap();
                let v = channel.channel.recv().await;
                match v {
                    Some(v) => match v {
                        NodeResponse::Shared(value) => value,
                        _ => {
                            todo!()
                        }
                    },
                    None => {
                        todo!()
                    }
                }
            }
            None => {
                let default = default.read().unwrap();
                Box::new(
                    self.read_fn
                        .call(&default, &mut self.memory, envelope_data, frame),
                )
            }
        }
    }

    fn set_channel(&mut self, channel: InputChannel, upstream_socket_id: SocketId) {
        self.channel = Some(InputSocketChannel {
            channel,
            upstream_socket_id,
        });
    }

    fn disconnect_channel(&mut self) -> Result<(), NodeDisconnectError> {
        match self.channel {
            Some(_) => {
                self.channel = None;
                Ok(())
            }
            None => Err(NodeDisconnectError::NotConnected),
        }
    }

    fn get_upstream_socket_id(&self) -> Option<&SocketId> {
        match self.channel {
            Some(ref channel) => Some(&channel.upstream_socket_id),
            None => None,
        }
    }

    async fn recv(&mut self) -> Option<NodeResponse> {
        self.channel.as_mut().unwrap().channel.recv().await
    }
}

/// # Input
pub struct Input<DATA, MEMORY, OUTPUT>
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    default: Arc<RwLock<DATA>>,
    data: Option<SharedAny>,
    envelope_data: Envelope,
    envelope_frame: Envelope,
    socket: InputSocket<DATA, MEMORY, OUTPUT>,
    id: SocketId,
}

impl<DATA, MEMORY, OUTPUT> Input<DATA, MEMORY, OUTPUT>
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    pub fn new_all(
        default: DATA,
        memory: MEMORY,
        envelope_data: Envelope,
        envelope_frame: Envelope,
        read: Box<dyn ReadFn<DATA, MEMORY, OUTPUT>>,
        frame: Box<dyn FrameFn>,
    ) -> Self {
        Input {
            default: Arc::new(RwLock::new(default)),
            data: None,
            envelope_data,
            envelope_frame,
            socket: InputSocket {
                channel: None,
                read_fn: read,
                memory,
                frame_fn: frame,
            },
            id: SocketId::new(),
        }
    }

    /// build chain new
    pub fn new(default: DATA, memory: MEMORY, read: Box<dyn ReadFn<DATA, MEMORY, OUTPUT>>) -> Self {
        Input {
            default: Arc::new(RwLock::new(default)),
            data: None,
            envelope_data: Envelope {},
            envelope_frame: Envelope {},
            socket: InputSocket {
                channel: None,
                read_fn: read,
                memory,
                frame_fn: Box::new(|frame: FrameCount, _: &Envelope| -> FrameCount { frame }),
            },
            id: SocketId::new(),
        }
    }

    /// build chain
    pub fn envelope_data(mut self, envelope_data: Envelope) -> Self {
        self.envelope_data = envelope_data;
        self
    }

    /// build chain
    pub fn envelope_frame(mut self, envelope_frame: Envelope) -> Self {
        self.envelope_frame = envelope_frame;
        self
    }

    /// build chain
    pub fn fn_frame(mut self, fn_frame: Box<dyn FrameFn>) -> Self {
        self.socket.frame_fn = fn_frame;
        self
    }

    /// update default data
    pub fn update_default(&self, update_default: SharedAny) -> Result<(), UpdateInputDefaultError> {
        match update_default.downcast::<DATA>() {
            Ok(update) => {
                let mut default = self.default.write().unwrap();
                *default = *update;
                Ok(())
            }
            Err(err) => Err(UpdateInputDefaultError::TypeRejected(err)),
        }
    }
}

/// # InputCommon
#[async_trait::async_trait]
pub trait InputCommon: Send + Sync + 'static {
    fn get_id(&self) -> &SocketId;
    // get data
    async fn get_clone(&mut self, frame: FrameCount) -> SharedAny;
    async fn get_ref<'a>(&'a mut self, frame: FrameCount) -> &'a SharedAny;
    // connect and disconnect
    async fn connect(&mut self, channel: InputChannel) -> Result<(), NodeConnectError>;
    async fn recv_connection_checker(&mut self) -> Result<Uuid, NodeConnectionCheckError>;
    fn disconnect(&mut self) -> Result<(), NodeDisconnectError>;
    // get upstream socket id
    fn get_upstream_socket_id(&self) -> Option<&SocketId>;
    // update default
    fn update_default(&self, update_default: SharedAny) -> Result<(), UpdateInputDefaultError>;
    // check cache delete request
    async fn check_cache_delete_request(&mut self) -> bool;
}

#[async_trait::async_trait]
impl<DATA, MEMORY, OUTPUT> InputCommon for Input<DATA, MEMORY, OUTPUT>
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    fn get_id(&self) -> &SocketId {
        &self.id
    }

    async fn get_clone(&mut self, frame: FrameCount) -> SharedAny {
        // boot socket
        self.socket
            .transcript(
                &self.default,
                &self.envelope_data,
                &self.envelope_frame,
                frame,
            )
            .await
    }

    async fn get_ref<'a>(&'a mut self, frame: FrameCount) -> &'a SharedAny {
        // boot socket
        self.data = Some(
            self.socket
                .transcript(
                    &self.default,
                    &self.envelope_data,
                    &self.envelope_frame,
                    frame,
                )
                .await,
        );
        // return
        self.data.as_ref().expect("")
    }

    async fn connect(&mut self, mut channel: InputChannel) -> Result<(), NodeConnectError> {
        // 1. receive typeid
        if let Some(response) = channel.recv().await {
            // 2. check typeid
            if let NodeResponse::CompatibleCheck {
                type_id,
                upstream_socket_id,
            } = response
            {
                if type_id != TypeId::of::<OUTPUT>() {
                    channel.send(NodeOrder::TypeRejected).await.unwrap();
                    return Err(NodeConnectError::TypeRejected);
                } else {
                    channel.send(NodeOrder::TypeConformed).await.unwrap();
                    self.socket.set_channel(channel, upstream_socket_id);
                    Ok(())
                }
            } else {
                panic!("Input::connect | NodeResponse::TypeId is expected. but not.")
            }
        } else {
            panic!("Input::connect | channel.recv() returns None. which is unexpected.")
        }
    }

    async fn recv_connection_checker(&mut self) -> Result<Uuid, NodeConnectionCheckError> {
        match self.socket.recv().await {
            Some(response) => match response {
                NodeResponse::ConnectionChecker(token) => Ok(token),
                _ => todo!(),
            },
            None => Err(NodeConnectionCheckError::ChannelClosed),
        }
    }

    fn disconnect(&mut self) -> Result<(), NodeDisconnectError> {
        self.socket.disconnect_channel()?;
        Ok(())
    }

    fn get_upstream_socket_id(&self) -> Option<&SocketId> {
        self.socket.get_upstream_socket_id()
    }

    fn update_default(&self, update_default: SharedAny) -> Result<(), UpdateInputDefaultError> {
        self.update_default(update_default)
    }

    async fn check_cache_delete_request(&mut self) -> bool {
        if let Some(ref mut input_channel) = self.socket.channel {
            let mut is_delete_cache_request = false;
            loop {
                match input_channel.channel.try_recv() {
                    Ok(message) => match message {
                        NodeResponse::DeleteCache => is_delete_cache_request = true,
                        _ => panic!(),
                    },
                    Err(err) => match err {
                        mpsc::error::TryRecvError::Empty => break,
                        mpsc::error::TryRecvError::Disconnected => todo!(),
                    },
                };
            }
            is_delete_cache_request
        } else {
            false
        }
    }
}



pub enum InputTree {
    Vec(Vec<InputTree>),
    Reef(Box<dyn InputCommon>),
}

impl std::ops::Index<usize> for InputTree {
    type Output = InputTree;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            InputTree::Vec(v) => &v[index],
            InputTree::Reef(_) => panic!("InputTree::Reef cannot be indexed"),
        }
    }
}

impl std::ops::IndexMut<usize> for InputTree {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            InputTree::Vec(v) => &mut v[index],
            InputTree::Reef(_) => panic!("InputTree::Reef cannot be indexed"),
        }
    }
}

impl From<Box<dyn InputCommon>> for InputTree {
    fn from(input: Box<dyn InputCommon>) -> Self {
        InputTree::Reef(input)
    }
}

impl InputTree {
    pub fn new_reef(input: Box<dyn InputCommon>) -> Self {
        InputTree::Reef(input)
    }

    pub fn new_vec(v: Vec<InputTree>) -> Self {
        InputTree::Vec(v)
    }

    pub fn push(&mut self, input: InputTree) {
        match self {
            InputTree::Vec(vec) => {
                if vec.len() == 0 {
                    panic!("0 length InputTree::Vec cannot be pushed");
                }
                vec.push(input)
            }
            InputTree::Reef(_) => panic!("InputTree::Reef cannot be pushed"),
        }
    }

    pub fn insert(&mut self, index: usize, input: InputTree) {
        match self {
            InputTree::Vec(vec) => {
                if vec.len() == 0 {
                    panic!("0 length InputTree::Vec cannot be inserted");
                }
                vec.insert(index, input)
            }
            InputTree::Reef(_) => panic!("InputTree::Reef cannot be inserted"),
        }
    }

    pub fn remove(&mut self, index: usize) -> InputTree {
        match self {
            InputTree::Vec(v) => v.remove(index),
            InputTree::Reef(_) => panic!("InputTree::Reef cannot be removed"),
        }
    }

    pub fn get_from_id(&mut self, id: &SocketId) -> Result<&mut Box<dyn InputCommon>, ()> {
        match self {
            InputTree::Vec(v) => {
                for input in v {
                    if let Ok(socket) = input.get_from_id(id) {
                        return Ok(socket);
                    }
                }
                Err(())
            }
            InputTree::Reef(ref mut socket) => {
                if socket.get_id() == id {
                    Ok(socket)
                } else {
                    Err(())
                }
            }
        }
    }

    pub fn reef(&mut self) -> &mut Box<dyn InputCommon> {
        match self {
            InputTree::Vec(_) => panic!("InputTree: is not Reef"),
            InputTree::Reef(socket) => socket,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            InputTree::Vec(v) => v.len(),
            InputTree::Reef(_) => 0,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            InputTree::Vec(v) => {
                let mut len = 0;
                for input in v {
                    len += input.size();
                }
                len
            }
            InputTree::Reef(_) => 1,
        }
    }

    pub async fn check_cache_delete_request(&mut self) -> bool {
        match self {
            InputTree::Vec(v) => {
                for input in v {
                    if Box::pin(input.check_cache_delete_request()).await {
                        return true;
                    }
                }
                false
            }
            InputTree::Reef(socket) => socket.check_cache_delete_request().await,
        }
    }
}