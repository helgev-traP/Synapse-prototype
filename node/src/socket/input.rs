use core::panic;
use std::{
    any::Any,
    sync::{Arc, Weak},
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, UpdateInputDefaultError,
        UpdateInputEnvelopeError,
    },
    node_core::NodeCore,
    socket::output::OutputCommon,
    types::{Envelope, SharedAny, SocketId},
    FrameCount,
};

/// # ReadFn
// todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
pub trait ReadingFn<'a, DATA, MEMORY, OUTPUT>: Send + Sync
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    fn call(
        &'a self,
        default: Option<&'a DATA>,
        memory: &'a mut MEMORY,
        envelope: Option<&'a Envelope>,
        frame: FrameCount,
    ) -> SharedAny;
}

impl<'a, DATA, MEMORY, OUTPUT, F> ReadingFn<'a, DATA, MEMORY, OUTPUT> for F
where
    DATA: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
    F: Fn(Option<&'a DATA>, &'a mut MEMORY, Option<&'a Envelope>, FrameCount) -> SharedAny
        + Send
        + Sync,
{
    fn call(
        &'a self,
        default: Option<&'a DATA>,
        memory: &'a mut MEMORY,
        envelope: Option<&'a Envelope>,
        frame: FrameCount,
    ) -> SharedAny {
        self(default, memory, envelope, frame)
    }
}

// Archive: 廃止
// /// # FrameFn
// // todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
// pub trait FrameSelectFn: Send + Sync {
//     fn call(&self, frame: FrameCount, envelope: &Envelope) -> FrameCount;
// }
//
// impl<F> FrameSelectFn for F
// where
//     F: Fn(FrameCount, &Envelope) -> FrameCount + Send + Sync,
// {
//     fn call(&self, frame: FrameCount, envelope: &Envelope) -> FrameCount {
//         self(frame, envelope)
//     }
// }

/// # InputSocket
// struct InputSocket<DATA, MEMORY, OUTPUT>
// where
//     DATA: Send + Sync + 'static,
//     MEMORY: Send + Sync + 'static,
//     OUTPUT: Send + Sync + 'static,
// {
// }
//
// impl<DATA, MEMORY, OUTPUT> InputSocket<DATA, MEMORY, OUTPUT>
// where
//     DATA: Send + Sync + 'static,
//     MEMORY: Send + Sync + 'static,
//     OUTPUT: Send + Sync + 'static,
// {
//     async fn transcript(
//         &mut self,
//         default: &Arc<RwLock<DATA>>,
//         envelope_data: &Envelope,
//         envelope_frame: &Envelope,
//         frame: FrameCount,
//     ) -> SharedAny {
//         todo!()
//     }
//
//     fn set_channel(&mut self, channel: InputChannel, upstream_socket_id: SocketId) {
//         todo!()
//     }
//
//     fn disconnect_channel(&mut self) -> Result<(), NodeDisconnectError> {
//         todo!()
//     }
//
//     fn get_upstream_socket_id(&self) -> Option<&SocketId> {
//         todo!()
//     }
//
//     async fn recv(&mut self) -> Option<NodeResponse> {
//         todo!()
//     }
// }

/// # Input

struct InputInner<Data, Memory, Output, NodeMemory, NodeOutput>
where
    Data: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: std::any::Any + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeOutput: Clone + Send + Sync + 'static,
{
    // The fields without interior mutability are only be assigned once
    // in the constructor of Input and never be changed.
    // identifier
    id: SocketId,
    name: String,

    // main body of node
    node: Arc<NodeCore<NodeOutput, NodeMemory>>,

    // from default value
    default_value_name: String,
    default_value: Option<RwLock<Data>>,
    envelope_name: String,
    envelope: Option<RwLock<Envelope>>,

    memory: RwLock<Memory>,
    reading_fn: Box<dyn for<'a> ReadingFn<'a, Data, Memory, Output>>,

    // from upstream node
    frame_select_envelope: RwLock<Envelope>,

    // upstream socket
    upstream_socket: RwLock<Option<Weak<dyn OutputCommon>>>,

    // cache
    output_cache: RwLock<Option<(FrameCount, SharedAny)>>,
}

pub struct Input<Data, Memory, Output, NodeMemory, NodeOutput>(
    Arc<InputInner<Data, Memory, Output, NodeMemory, NodeOutput>>,
)
where
    Data: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeOutput: Clone + Send + Sync + 'static;

/// build chain
impl<Default, Memory, Output, NodeMemory, NodeOutput>
    Input<Default, Memory, Output, NodeMemory, NodeOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeOutput: Clone + Send + Sync + 'static,
{
    pub fn new(
        name: String,
        node: Arc<NodeCore<NodeOutput, NodeMemory>>,
        default_value_name: String,
        default_value: Option<Default>,
        envelope_name: String,
        envelope: Option<Envelope>,
        memory: Memory,
        reading_fn: Box<dyn for<'a> ReadingFn<'a, Default, Memory, Output>>,
        frame_select_envelope: Envelope,
    ) -> Self {
        Input(Arc::new(InputInner {
            id: SocketId::new(),
            name,
            node,
            default_value_name,
            default_value: default_value.map(|data| RwLock::new(data)),
            envelope_name,
            envelope: envelope.map(|data| RwLock::new(data)),
            memory: RwLock::new(memory),
            reading_fn,
            frame_select_envelope: RwLock::new(frame_select_envelope),
            upstream_socket: RwLock::new(None),
            output_cache: RwLock::new(None),
        }))
    }

    pub fn weak(&self) -> Weak<dyn InputCommon> {
        Arc::downgrade(&self.0) as Weak<dyn InputCommon>
    }
}

impl<Default, Memory, Output, NodeMemory, NodeOutput>
    Input<Default, Memory, Output, NodeMemory, NodeOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeOutput: Clone + Send + Sync + 'static,
{
}

/// # InputCommon

#[async_trait::async_trait]
impl<Data, Memory, Output, NodeMemory, NodeOutput> InputCommon
    for Input<Data, Memory, Output, NodeMemory, NodeOutput>
where
    Data: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Any + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeOutput: Clone + Send + Sync + 'static,
{
    // getters
    fn get_id(&self) -> &SocketId {
        &self.0.id
    }

    fn get_name(&self) -> &str {
        &self.0.name
    }

    fn get_default_value_name(&self) -> &str {
        &self.0.default_value_name
    }

    fn get_envelope_name(&self) -> &str {
        &self.0.envelope_name
    }

    // get / update default value / envelope
    async fn get_default_value(&self) -> Option<&dyn Any> {
        Some(&*self.0.default_value.as_ref()?.read().await)
    }

    async fn get_envelope(&self) -> Option<&Envelope> {
        Some(&*self.0.envelope.as_ref()?.read().await)
    }

    async fn set_default_value(
        &self,
        default_value: SharedAny,
    ) -> Result<(), UpdateInputDefaultError> {
        match default_value.downcast::<Data>() {
            Ok(data) => {
                if let Some(default_value) = &self.0.default_value {
                    *default_value.write().await = *data;
                    Ok(())
                } else {
                    Err(UpdateInputDefaultError::DefaultValueNotEnabled(
                        default_value,
                    ))
                }
            }
            Err(_) => Err(UpdateInputDefaultError::TypeRejected(default_value)),
        }
    }

    async fn set_envelope(&self, envelope: Envelope) -> Result<(), UpdateInputEnvelopeError> {
        if let Some(envelope_entity) = &self.0.envelope {
            *envelope_entity.write().await = envelope;
            Ok(())
        } else {
            Err(UpdateInputEnvelopeError::EnvelopeNotEnabled(envelope))
        }
    }

    // frame selection envelope
    async fn get_frame_select_envelope(&self) -> &Envelope {
        &*self.0.frame_select_envelope.read().await
    }

    async fn set_frame_select_envelope(&self, envelope: Envelope) {
        *self.0.frame_select_envelope.write().await = envelope;
    }

    // --- communications ---
    // get data
    async fn get_ref(&mut self, frame: FrameCount) -> &SharedAny {
        match self.0.upstream_socket.read().await.as_ref() {
            Some(socket) => {
                // determine frame
                let frame = self.get_frame_select_envelope().await.value() as FrameCount;

                if let Some((cache_frame, cache_data)) = &*self.0.output_cache.read().await {
                    // if cache hit
                    if *cache_frame == frame {
                        return cache_data;
                    }
                }

                // if cache miss
                // get data from upstream
                let data = socket.upgrade().unwrap().call().downcast().unwrap();

                // update cache

                self.0.output_cache.write().await.replace((frame, *data));

                &self.0.output_cache.read().await.as_ref().unwrap().1
            }
            None => {
                let default = if let Some(default) = self.0.default_value.as_ref() {
                    Some(&*default.read().await)
                } else {
                    None
                };

                let envelope = if let Some(envelope) = self.0.envelope.as_ref() {
                    Some(&*envelope.read().await)
                } else {
                    None
                };

                self.0.output_cache.write().await.replace((
                    frame,
                    self.0.reading_fn.call(
                        default,
                        &mut *self.0.memory.write().await,
                        envelope,
                        frame,
                    ),
                ));

                &self.0.output_cache.read().await.as_ref().unwrap().1
            }
        }
    }

    // get upstream socket id
    async fn get_upstream_socket_id(&self) -> Option<&SocketId> {
        Some(
            self.0
                .upstream_socket
                .read()
                .await
                .as_ref()?
                .upgrade()
                .unwrap()
                .get_id(),
        )
    }

    // connect and disconnect
    async fn connect(&mut self, socket: Arc<dyn OutputCommon>) -> Result<(), NodeConnectError> {
        todo!()
    }

    async fn recv_connection_checker(&mut self) -> Result<Uuid, NodeConnectionCheckError> {
        todo!()
    }

    async fn disconnect(&mut self) -> Result<(), NodeDisconnectError> {
        if let Some(socket) = self.0.upstream_socket.read().await.as_ref() {
            todo!()
        } else {
            Err(NodeDisconnectError::NotConnected)
        }
    }

    // clone self
    fn clone_self(&self) -> Box<dyn InputCommon> {
        Box::new(self.clone_self())
    }
}

#[async_trait::async_trait]
pub trait InputCommon: Send + Sync + 'static {
    // getters
    fn get_id(&self) -> &SocketId;
    fn get_name(&self) -> &str;
    fn get_default_value_name(&self) -> &str;
    fn get_envelope_name(&self) -> &str;

    // get / update default value / envelope
    async fn get_default_value(&self) -> Option<&dyn Any>;
    async fn get_envelope(&self) -> Option<&Envelope>;

    async fn set_default_value(
        &self,
        default_value: SharedAny,
    ) -> Result<(), UpdateInputDefaultError>;
    async fn set_envelope(&self, envelope: Envelope) -> Result<(), UpdateInputEnvelopeError>;

    // frame selection envelope
    async fn get_frame_select_envelope(&self) -> &Envelope;
    async fn set_frame_select_envelope(&self, envelope: Envelope);

    // --- communications ---
    // get data
    async fn get_ref(&mut self, frame: FrameCount) -> &SharedAny;

    // get upstream socket id
    async fn get_upstream_socket_id(&self) -> Option<&SocketId>;

    // connect and disconnect
    async fn connect(&mut self, socket: Arc<dyn OutputCommon>) -> Result<(), NodeConnectError>;
    async fn recv_connection_checker(&mut self) -> Result<Uuid, NodeConnectionCheckError>;
    async fn disconnect(&mut self) -> Result<(), NodeDisconnectError>;

    // clone self
    fn weak(&self) -> Box<dyn InputCommon>;
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
}
