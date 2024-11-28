use std::{
    any::Any,
    sync::{Arc, Weak},
};
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::{
    err::{
        NodeDisconnectError, UpdateInputDefaultError, UpdateInputEnvelopeError,
    },
    node_core::{NodeCore, NodeCoreCommon},
    socket::output::OutputTrait,
    types::{Envelope, SharedAny, SocketId},
    FrameCount,
};

/// # ReadFn
// todo クロージャを宣言するときに無駄な引数を取る必要が無いようにしたい
pub trait ReadingFn<'a, DEFAULT, MEMORY, OUTPUT>: Send + Sync
where
    DEFAULT: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
{
    fn call(
        &'a self,
        default: Option<&'a DEFAULT>,
        memory: &'a mut MEMORY,
        envelope: Option<&'a Envelope>,
        frame: FrameCount,
    ) -> OUTPUT;
}

impl<'a, DEFAULT, MEMORY, OUTPUT, F> ReadingFn<'a, DEFAULT, MEMORY, OUTPUT> for F
where
    DEFAULT: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
    F: Fn(Option<&'a DEFAULT>, &'a mut MEMORY, Option<&'a Envelope>, FrameCount) -> OUTPUT
        + Send
        + Sync,
{
    fn call(
        &'a self,
        default: Option<&'a DEFAULT>,
        memory: &'a mut MEMORY,
        envelope: Option<&'a Envelope>,
        frame: FrameCount,
    ) -> OUTPUT {
        self(default, memory, envelope, frame)
    }
}

/// # Input

pub struct InputSocket<'a, Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    // The fields without interior mutability are only be assigned once
    // in the constructor of Input and never be changed.
    // identifier
    id: SocketId,
    name: String,

    // main body of node
    node: Arc<NodeCore<'a, NodeInputs, NodeMemory, NodeProcessOutput>>,

    // from default value
    default_value_name: String,
    default_value: Option<RwLock<Default>>,
    envelope_name: String,
    envelope: Option<RwLock<Envelope>>,

    memory: RwLock<Memory>,
    reading_fn: Box<dyn for<'b> ReadingFn<'b, Default, Memory, Output>>,

    // from upstream node
    frame_select_envelope: RwLock<Envelope>,

    // upstream socket
    upstream_socket: RwLock<Option<Weak<dyn OutputTrait>>>,
}

/// build chain
impl<'a, Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
    InputSocket<'a, Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    pub fn new(
        name: String,
        node: Arc<NodeCore<'a, NodeInputs, NodeMemory, NodeProcessOutput>>,
        default_value_name: String,
        default_value: Option<Default>,
        envelope_name: String,
        envelope: Option<Envelope>,
        memory: Memory,
        reading_fn: Box<dyn for<'b> ReadingFn<'b, Default, Memory, Output>>,
        frame_select_envelope: Envelope,
    ) -> Self {
        InputSocket {
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
        }
    }
}

/// get socket value
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
    InputSocket<'_, Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    pub async fn get(&mut self, frame: FrameCount) -> Output {
        match self.upstream_socket.read().await.as_ref() {
            Some(socket) => {
                // determine frame
                let frame = self.frame_select_envelope.read().await.value() as FrameCount;

                // get data from upstream
                let data = socket
                    .upgrade()
                    .unwrap()
                    .call(frame, self.id)
                    .await
                    .downcast()
                    .unwrap();

                *data
            }
            None => {
                let default_rw_guard;
                let default = match self.default_value.as_ref() {
                    Some(data) => {
                        default_rw_guard = data.read().await;
                        Some(&*default_rw_guard)
                    }
                    None => None,
                };

                let envelope_rw_guard;
                let envelope = match self.envelope.as_ref() {
                    Some(data) => {
                        envelope_rw_guard = data.read().await;
                        Some(&*envelope_rw_guard)
                    }
                    None => None,
                };

                self.reading_fn
                    .call(default, &mut *self.memory.write().await, envelope, frame)
            }
        }
    }
}

/// # InputCommon

#[async_trait::async_trait]
impl<'a, Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput> InputTrait
    for InputSocket<'a, Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    // --- use from NodeField ---
    // getters
    fn get_id(&self) -> SocketId {
        self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_default_value_name(&self) -> &str {
        &self.default_value_name
    }

    fn get_envelope_name(&self) -> &str {
        &self.envelope_name
    }

    // get / update default value / envelope
    async fn get_default_value<'b>(&'b self) -> Option<RwLockReadGuard<'b, dyn Any>> {
        // todo
        todo!()
    }

    async fn get_envelope<'b>(&'b self) -> Option<RwLockReadGuard<'b, Envelope>> {
        Some(self.envelope.as_ref()?.read().await)
    }

    async fn set_default_value(
        &self,
        default_value: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError> {
        match default_value.downcast::<Default>() {
            Ok(data) => {
                if let Some(default_value) = &self.default_value {
                    *default_value.write().await = *data;
                    Ok(())
                } else {
                    Err(UpdateInputDefaultError::DefaultValueNotEnabled(
                        data as Box<dyn Any + Send + Sync + 'static>,
                    ))
                }
            }
            Err(e) => Err(UpdateInputDefaultError::TypeRejected(e)),
        }
    }

    async fn set_envelope(&self, envelope: Envelope) -> Result<(), UpdateInputEnvelopeError> {
        if let Some(envelope_entity) = &self.envelope {
            *envelope_entity.write().await = envelope;
            Ok(())
        } else {
            Err(UpdateInputEnvelopeError::EnvelopeNotEnabled(envelope))
        }
    }

    // frame selection envelope
    async fn get_frame_select_envelope<'b>(&'b self) -> RwLockReadGuard<'b, Envelope> {
        self.frame_select_envelope.read().await
    }

    async fn set_frame_select_envelope(&self, envelope: Envelope) {
        *self.frame_select_envelope.write().await = envelope;
    }

    // get upstream socket id
    async fn get_upstream_socket_id(&self) -> Option<SocketId> {
        Some(
            self.upstream_socket
                .read()
                .await
                .as_ref()?
                .upgrade()
                .unwrap()
                .get_id(),
        )
    }

    // connect and disconnect
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Output>()
    }

    async fn connect(&self, socket: Weak<dyn OutputTrait>) {
        *self.upstream_socket.write().await = Some(socket);
    }

    async fn disconnect(&self) -> Result<(), NodeDisconnectError> {
        let mut socket_holder = self.upstream_socket.write().await;

        let Some(socket) = socket_holder.as_ref() else {
            return Err(NodeDisconnectError::NotConnected);
        };

        let socket = socket.upgrade().unwrap();

        // disconnect
        if let Err(e) = socket.disconnect(self.id).await {
            if let NodeDisconnectError::NotConnected = e {
                println!("NodeDisconnectError: Data inconsistency occurred.");
            }
        }

        // clear upstream socket
        *socket_holder = None;

        Ok(())
    }

    // --- use from OutputSocket ---
    async fn clear_cache(&self) {
        self.node.clear_cache().await;
    }
}

#[async_trait::async_trait]
pub(crate) trait InputTrait: Send + Sync {
    // --- use from NodeField ---
    // getters
    fn get_id(&self) -> SocketId;
    fn get_name(&self) -> &str;
    fn get_default_value_name(&self) -> &str;
    fn get_envelope_name(&self) -> &str;

    // get / update default value / envelope
    async fn get_default_value<'a>(&'a self) -> Option<RwLockReadGuard<'a, dyn Any>>;
    async fn get_envelope<'a>(&'a self) -> Option<RwLockReadGuard<'a, Envelope>>;

    async fn set_default_value(
        &self,
        default_value: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError>;
    async fn set_envelope(&self, envelope: Envelope) -> Result<(), UpdateInputEnvelopeError>;

    // frame selection envelope
    async fn get_frame_select_envelope<'a>(&'a self) -> RwLockReadGuard<'a, Envelope>;
    async fn set_frame_select_envelope(&self, envelope: Envelope);

    // get upstream socket id
    async fn get_upstream_socket_id(&self) -> Option<SocketId>;

    // connect and disconnect
    fn type_id(&self) -> std::any::TypeId;
    async fn connect(&self, socket: Weak<dyn OutputTrait>);
    async fn disconnect(&self) -> Result<(), NodeDisconnectError>;

    // --- use from OutputSocket ---
    async fn clear_cache(&self);
}

#[async_trait::async_trait]
pub trait InputGroup: Send + 'static {
    async fn get_socket(&self, id: &SocketId) -> Option<Weak<dyn InputTrait>>;
}
