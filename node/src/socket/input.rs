use core::panic;
use std::{
    any::Any,
    sync::{Arc, Weak},
};
use tokio::sync::{RwLock, RwLockReadGuard};

use envelope::Envelope;

use crate::{
    err::{NodeDisconnectError, UpdateInputDefaultError, UpdateInputEnvelopeError},
    node_core::{Node, NodeCoreCommon},
    socket::OutputSocketCapsule,
    types::{SharedAny, SocketId},
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
        envelope: Option<&'a Envelope>,
        memory: &'a mut MEMORY,
        frame: FrameCount,
    ) -> OUTPUT;
}

impl<'a, DEFAULT, MEMORY, OUTPUT, F> ReadingFn<'a, DEFAULT, MEMORY, OUTPUT> for F
where
    DEFAULT: Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
    OUTPUT: Send + Sync + 'static,
    F: Fn(Option<&'a DEFAULT>, Option<&'a Envelope>, &'a mut MEMORY, FrameCount) -> OUTPUT
        + Send
        + Sync,
{
    fn call(
        &'a self,
        default: Option<&'a DEFAULT>,
        envelope: Option<&'a Envelope>,
        memory: &'a mut MEMORY,
        frame: FrameCount,
    ) -> OUTPUT {
        self(default, envelope, memory, frame)
    }
}

/// # Input
pub struct InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    // Weak to self
    weak: Weak<InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>>,

    // The fields without interior mutability are only be assigned once
    // in the constructor of Input and never be changed.
    // identifier
    id: SocketId,
    name: String,

    // main body of node
    node: Weak<Node<NodeInputs, NodeMemory, NodeProcessOutput>>,

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
    upstream_socket: RwLock<Option<OutputSocketCapsule>>,
}

/// Constructor for InputSocket
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
    InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    pub fn new(
        name: &str,
        node: &Weak<Node<NodeInputs, NodeMemory, NodeProcessOutput>>,
        default_value_name: &str,
        default_value: Option<Default>,
        envelope_name: &str,
        envelope: Option<Envelope>,
        memory: Memory,
        reading_fn: Box<dyn for<'b> ReadingFn<'b, Default, Memory, Output>>,
        frame_select_envelope: Option<Envelope>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak| InputSocket {
            weak: weak.clone(),
            id: SocketId::new(),
            name: name.to_string(),
            node: node.clone(),
            default_value_name: default_value_name.to_string(),
            default_value: default_value.map(RwLock::new),
            envelope_name: envelope_name.to_string(),
            envelope: envelope.map(RwLock::new),
            memory: RwLock::new(memory),
            reading_fn,
            frame_select_envelope: RwLock::new(
                frame_select_envelope.unwrap_or_else(Envelope::new_pass_through),
            ),
            upstream_socket: RwLock::new(None),
        })
    }
}

impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
    InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    pub fn make_capsule(&self) -> InputSocketCapsule {
        InputSocketCapsule {
            socket_id: self.id,
            socket_type: std::any::TypeId::of::<Output>(),
            weak: self.weak.clone() as Weak<dyn InputSocketTrait>,
        }
    }
}

/// Get the value of socket
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
    InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    pub async fn get(&self, frame: FrameCount) -> Output {
        match self.upstream_socket.read().await.as_ref() {
            Some(socket) => {
                // determine frame
                let frame =
                    self.frame_select_envelope.read().await.value(frame as f64) as FrameCount;

                // get data from upstream
                let data = socket.call(frame).await.downcast().unwrap();

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
                    .call(default, envelope, &mut *self.memory.write().await, frame)
            }
        }
    }
}

/// Getter/Setter that be used in NodeCore
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
    InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    // getters
    pub fn get_id(&self) -> SocketId {
        self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_default_value_name(&self) -> &str {
        &self.default_value_name
    }

    pub fn get_envelope_name(&self) -> &str {
        &self.envelope_name
    }

    // get / update default value / envelope
    pub async fn get_default_value<'b>(&'b self) -> Option<RwLockReadGuard<'b, dyn Any>> {
        // todo
        todo!()
    }

    pub async fn get_envelope<'b>(&'b self) -> Option<RwLockReadGuard<'b, Envelope>> {
        Some(self.envelope.as_ref()?.read().await)
    }

    // frame selection envelope
    pub async fn get_frame_select_envelope<'b>(&'b self) -> RwLockReadGuard<'b, Envelope> {
        self.frame_select_envelope.read().await
    }

    pub async fn set_frame_select_envelope(&self, envelope: Envelope) {
        *self.frame_select_envelope.write().await = envelope;
    }

    // get upstream socket id
    pub async fn get_upstream_socket_id(&self) -> Option<SocketId> {
        Some(self.upstream_socket.read().await.as_ref()?.socket_id())
    }

    // connect and disconnect
    pub fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Output>()
    }
}

#[async_trait::async_trait]
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput> InputSocketCommon
    for InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    async fn get_upstream_socket_id(&self) -> Option<SocketId> {
        InputSocket::get_upstream_socket_id(self).await
    }

    async fn set_default_value(
        &self,
        default_value: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError> {
        match default_value.downcast::<Default>() {
            Ok(data) => {
                if let Some(default_value) = &self.default_value {
                    *default_value.write().await = *data;

                    // clear cache
                    self.clear_cache().await;

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

            // clear cache
            self.clear_cache().await;

            Ok(())
        } else {
            Err(UpdateInputEnvelopeError::EnvelopeNotEnabled(envelope))
        }
    }

    async fn connect(&self, socket: OutputSocketCapsule) {
        *self.upstream_socket.write().await = Some(socket);

        // clear cache
        self.clear_cache().await;
    }

    async fn disconnect(&self) -> Result<(), NodeDisconnectError> {
        // make upstream socket disconnect
        let mut socket_holder = self.upstream_socket.write().await;
        let Some(socket) = socket_holder.as_ref() else {
            return Err(NodeDisconnectError::NotConnected);
        };
        if let Err(NodeDisconnectError::NotConnected) = socket.disconnect(self.id).await {
            println!("NodeDisconnectError: Data inconsistency occurred.");
        }

        // clear upstream socket
        *socket_holder = None;

        // clear cache
        self.clear_cache().await;

        Ok(())
    }
}

#[async_trait::async_trait]
trait InputSocketCommon: Send + Sync {
    // set default value and envelope
    async fn set_default_value(
        &self,
        default_value: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError>;
    async fn set_envelope(&self, envelope: Envelope) -> Result<(), UpdateInputEnvelopeError>;
    // methods used to connect and disconnect
    async fn get_upstream_socket_id(&self) -> Option<SocketId>;
    async fn connect(&self, socket: OutputSocketCapsule);
    async fn disconnect(&self) -> Result<(), NodeDisconnectError>;
}

#[async_trait::async_trait]
impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput> InputSocketApi
    for InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
    async fn disconnect_called_from_upstream(&self) -> Result<(), NodeDisconnectError> {
        *self.upstream_socket.blocking_write() = None;

        // clear cache
        self.clear_cache().await;

        Ok(())
    }

    async fn clear_cache(&self) {
        let Some(node) = self.node.upgrade() else {
            panic!("InputSocket: NodeCore is not available. NodeCore stores sockets so it should not be None here. Doubt this Arc<{{socket}}> are cloned wrongly.");
        };

        node.clear_cache().await;
    }
}

#[async_trait::async_trait]
trait InputSocketApi: Send + Sync {
    /// Disconnect without calling OutputSocket::disconnect.
    /// This is used when OutputSocket::disconnect_all is called.
    async fn disconnect_called_from_upstream(&self) -> Result<(), NodeDisconnectError>;
    async fn clear_cache(&self);
}

impl<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput> InputSocketTrait
    for InputSocket<Default, Memory, Output, NodeInputs, NodeMemory, NodeProcessOutput>
where
    Default: Send + Sync + 'static,
    Memory: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    NodeInputs: InputGroup + Send + Sync + 'static,
    NodeMemory: Send + Sync + 'static,
    NodeProcessOutput: Clone + Send + Sync + 'static,
{
}

#[async_trait::async_trait]
trait InputSocketTrait: InputSocketCommon + InputSocketApi {}

/// A capsule of InputSocket that hide the type of its generics,
/// to transfer connection of InputSocket
pub struct InputSocketCapsule {
    socket_id: SocketId,
    socket_type: std::any::TypeId,
    weak: Weak<dyn InputSocketTrait>,
}

impl Clone for InputSocketCapsule {
    fn clone(&self) -> Self {
        Self {
            socket_id: self.socket_id,
            socket_type: self.socket_type,
            weak: self.weak.clone(),
        }
    }
}

impl InputSocketCapsule {
    pub fn socket_id(&self) -> SocketId {
        self.socket_id
    }

    pub fn socket_type(&self) -> std::any::TypeId {
        self.socket_type
    }

    pub async fn set_default_value(
        &self,
        default_value: Box<SharedAny>,
    ) -> Result<(), UpdateInputDefaultError> {
        // todo: add error handling when the weak reference is invalid

        if let Some(socket) = self.weak.upgrade() {
            socket.set_default_value(default_value).await
        } else {
            todo!()
        }
    }

    pub async fn set_envelope(&self, envelope: Envelope) -> Result<(), UpdateInputEnvelopeError> {
        // todo: add error handling when the weak reference is invalid

        if let Some(socket) = self.weak.upgrade() {
            socket.set_envelope(envelope).await
        } else {
            todo!()
        }
    }

    pub async fn clear_cache(&self) {
        // todo: add error handling when the weak reference is invalid

        if let Some(socket) = self.weak.upgrade() {
            socket.clear_cache().await;
        }
    }

    pub async fn upstream_socket_id(&self) -> Option<SocketId> {
        // todo: add error handling when the weak reference is invalid

        if let Some(socket) = self.weak.upgrade() {
            socket.get_upstream_socket_id().await
        } else {
            todo!()
        }
    }

    pub async fn connect(&self, socket: OutputSocketCapsule) {
        // todo: add error handling when the weak reference is invalid

        if let Some(i_socket) = self.weak.upgrade() {
            i_socket.connect(socket).await;
        }
    }

    pub async fn disconnect(&self) -> Result<(), NodeDisconnectError> {
        // todo: add error handling when the weak reference is invalid

        if let Some(socket) = self.weak.upgrade() {
            socket.disconnect().await
        } else {
            Err(NodeDisconnectError::NotConnected)
        }
    }

    pub async fn disconnect_called_from_upstream(&self) -> Result<(), NodeDisconnectError> {
        // todo : add error handling when the weak reference is invalid

        if let Some(socket) = self.weak.upgrade() {
            socket.disconnect_called_from_upstream().await
        } else {
            todo!()
        }
    }
}

#[async_trait::async_trait]
pub trait InputGroup: Send + 'static {
    async fn get_socket(&self, id: SocketId) -> Option<InputSocketCapsule>;
    fn get_all_socket(&self) -> Vec<InputSocketCapsule>;
}
