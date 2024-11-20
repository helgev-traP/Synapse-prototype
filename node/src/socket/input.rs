use std::{
    any::Any,
    sync::{Arc, Weak},
};
use tokio::sync::RwLock;

use crate::{
    err::{
        NodeConnectError, NodeDisconnectError, UpdateInputDefaultError, UpdateInputEnvelopeError,
    },
    node_core::NodeCore,
    socket::output::OutputCommon,
    types::{Envelope, SharedAny, SocketId},
    FrameCount,
};

use super::OutputTree;

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

struct InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: std::any::Any + Send + Sync + 'static,
    NodeInputs: InputTree + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
    NodeOutputs: OutputTree + Send + 'static,
{
    // The fields without interior mutability are only be assigned once
    // in the constructor of Input and never be changed.
    // identifier
    id: SocketId,
    name: String,

    // main body of node
    node: Arc<NodeCore<NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>>,

    // from default value
    default_value_name: String,
    default_value: Option<RwLock<Default>>,
    envelope_name: String,
    envelope: Option<RwLock<Envelope>>,

    memory: RwLock<Memory>,
    reading_fn: Box<dyn for<'a> ReadingFn<'a, Default, Memory, Output>>,

    // from upstream node
    frame_select_envelope: RwLock<Envelope>,

    // upstream socket
    upstream_socket: RwLock<Option<Weak<dyn OutputCommon>>>,
}

/// build chain
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
    InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: std::any::Any + Send + Sync + 'static,
    NodeInputs: InputTree + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
    NodeOutputs: OutputTree + Send + 'static,
{
    pub fn new(
        name: String,
        node: Arc<NodeCore<NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>>,
        default_value_name: String,
        default_value: Option<Default>,
        envelope_name: String,
        envelope: Option<Envelope>,
        memory: Memory,
        reading_fn: Box<dyn for<'a> ReadingFn<'a, Default, Memory, Output>>,
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
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
    InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: std::any::Any + Send + Sync + 'static,
    NodeInputs: InputTree + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
    NodeOutputs: OutputTree + Send + 'static,
{
    pub async fn get(&mut self, frame: FrameCount) -> Output {
        match self.upstream_socket.read().await.as_ref() {
            Some(socket) => {
                // determine frame
                let frame = self.get_frame_select_envelope().await.value() as FrameCount;

                // get data from upstream
                let data = socket
                    .upgrade()
                    .unwrap()
                    .call(frame)
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
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs> InputCommon
    for InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput, NodeOutputs>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: std::any::Any + Send + Sync + 'static,
    NodeInputs: InputTree + Send + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
    NodeOutputs: OutputTree + Send + 'static,
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
    async fn get_default_value(&self) -> Option<&dyn Any> {
        Some(&*self.default_value.as_ref()?)
    }

    async fn get_envelope(&self) -> Option<&Envelope> {
        Some(&*self.envelope.as_ref()?.read().await)
    }

    async fn set_default_value(
        &self,
        default_value: SharedAny,
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
    async fn get_frame_select_envelope(&self) -> &Envelope {
        &*self.frame_select_envelope.read().await
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
    async fn connect(&self, socket: Weak<dyn OutputCommon>) -> Result<(), NodeConnectError> {
        todo!()
    }

    async fn disconnect(&self) -> Result<(), NodeDisconnectError> {
        if let Some(socket) = self.upstream_socket.read().await.as_ref() {
            todo!()
        } else {
            Err(NodeDisconnectError::NotConnected)
        }
    }

    // --- use from OutputSocket ---
    async fn cache_clear(&self) {
        todo!()
    }
}

#[async_trait::async_trait]
pub(crate) trait InputCommon: Send + Sync + 'static {
    // --- use from NodeField ---
    // getters
    fn get_id(&self) -> SocketId;
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

    // get upstream socket id
    async fn get_upstream_socket_id(&self) -> Option<SocketId>;

    // connect and disconnect
    async fn connect(&self, socket: Weak<dyn OutputCommon>) -> Result<(), NodeConnectError>;
    async fn disconnect(&self) -> Result<(), NodeDisconnectError>;

    // --- use from OutputSocket ---
    async fn cache_clear(&self);
}

#[async_trait::async_trait]
pub trait InputTree: Send + 'static {
    async fn get_socket(&self, id: &SocketId) -> Option<Weak<dyn InputCommon>>;
}
