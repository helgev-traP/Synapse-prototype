use core::panic;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::{
    channel::{InputChannel, NodeOrder, NodeResponse, OutputChannel},
    err::{
        NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, NodeSendResponseError,
        UpdateInputDefaultError,
    },
    types::{Envelope, NodeId, SharedAny, SocketId, TryRecvResult},
    FrameCount,
};

pub struct Output<OUTPUT: Send + Sync + 'static> {
    id: SocketId,
    pickup: Box<dyn Fn(&OUTPUT) -> SharedAny + Send + Sync>,
    channels: HashMap<SocketId, OutputChannel>,
}

impl<OUTPUT: Send + Sync + 'static> Output<OUTPUT> {
    pub fn new(pickup: Box<dyn Fn(&OUTPUT) -> SharedAny + Send + Sync>) -> Self {
        Output {
            id: SocketId::new(),
            pickup,
            channels: HashMap::new(),
        }
    }

    pub fn get_id(&self) -> &SocketId {
        &self.id
    }

    pub async fn connect(
        &mut self,
        mut channel: OutputChannel,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        // 1. send typeid
        channel
            .send(NodeResponse::CompatibleCheck {
                type_id: TypeId::of::<OUTPUT>(),
                upstream_socket_id: self.id.clone(),
            })
            .await
            .unwrap();
        // 2. receive result of typeid check
        let recv_result = channel.recv().await;
        match recv_result {
            Some(node_order) => match node_order {
                NodeOrder::TypeConformed => {
                    self.channels.insert(downstream_socket_id.clone(), channel);
                    Ok(())
                }
                NodeOrder::TypeRejected => Err(NodeConnectError::TypeRejected),
                _ => panic!("Output::connect | NodeOrder::TypeConformed or NodeOrder::TypeRejected is expected. but not."),
            },
            _ => panic!("Output::connect | channel.recv() returns None. which is unexpected."),
        }
    }

    // 制作メモ
    // チャンネルの導通を確認するための物
    // 間違えて不適切な組のチャンネルをdropしないように確認できるようにする
    pub async fn send_connection_checker(
        &mut self,
        token: Uuid,
        socket_id: &SocketId,
    ) -> Result<(), NodeConnectionCheckError> {
        match self.channels.get(socket_id) {
            // socket id found
            Some(channel) => match channel.send(NodeResponse::ConnectionChecker(token)).await {
                Ok(_) => Ok(()),
                Err(_) => {
                    self.channels.remove(socket_id);
                    Err(NodeConnectionCheckError::ChannelClosed)
                }
            },
            // socket id not found
            None => Err(NodeConnectionCheckError::SocketIdNotFound),
        }
    }

    pub async fn disconnect(
        &mut self,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        match self.channels.get(downstream_socket_id) {
            Some(_) => {
                self.channels.remove(downstream_socket_id);
                Ok(())
            }
            None => Err(NodeDisconnectError::NotConnected),
        }
    }

    pub async fn get_destinations(&self) -> Option<(&NodeId, &SocketId)> {
        todo!()
    }

    pub async fn send_response(
        &mut self,
        output: &OUTPUT,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeSendResponseError> {
        match self.channels.get(downstream_socket_id) {
            Some(channel) => match channel
                .send(NodeResponse::Shared((self.pickup)(&output)))
                .await
            {
                Ok(_) => Ok(()),
                Err(_) => {
                    self.channels.remove(downstream_socket_id);
                    Err(NodeSendResponseError::ChannelClosed)
                }
            },
            None => Err(NodeSendResponseError::DownstreamNodeIdNotFound),
        }
    }

    pub async fn send_process_failed(
        &mut self,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeSendResponseError> {
        match self.channels.get(downstream_socket_id) {
            Some(channel) => match channel.send(NodeResponse::ProcessFailed).await {
                Ok(_) => Ok(()),
                Err(_) => {
                    self.channels.remove(downstream_socket_id);
                    Err(NodeSendResponseError::ChannelClosed)
                }
            },
            None => Err(NodeSendResponseError::DownstreamNodeIdNotFound),
        }
    }

    pub async fn send_delete_cache(&mut self) {
        let mut socket_disconnected = Vec::new();
        for (socket_id, channel) in self.channels.iter_mut() {
            if let Err(_) = channel.send(NodeResponse::DeleteCache).await {
                socket_disconnected.push(socket_id.clone());
            }
        }
        for socket_id in socket_disconnected {
            self.channels.remove(&socket_id);
        }
    }

    pub fn try_recv_request(&mut self) -> TryRecvResult {
        let mut requests = Vec::new();
        let mut socket_disconnected = Vec::new();
        // search all sockets
        for (socket_id, channel) in self.channels.iter_mut() {
            match channel.try_recv() {
                Ok(order) => {
                    requests.push((order, *socket_id));
                }
                Err(err) => match err {
                    mpsc::error::TryRecvError::Empty => {}
                    mpsc::error::TryRecvError::Disconnected => {
                        socket_disconnected.push(*socket_id);
                    }
                },
            }
        }
        // remove disconnected sockets
        for socket_id in socket_disconnected {
            self.channels.remove(&socket_id);
        }
        // return
        if requests.len() == 0 {
            TryRecvResult::Empty
        } else {
            TryRecvResult::Order(requests)
        }
    }
}

// todo: indexを削除
pub enum OutputTree<OUTPUT>
where
    OUTPUT: Clone + Send + Sync + 'static,
{
    Vec {
        vec: Vec<OutputTree<OUTPUT>>,
        index: Mutex<usize>,
    },
    Reef(Arc<Mutex<Output<OUTPUT>>>),
}

impl<OUTPUT> std::ops::Index<usize> for OutputTree<OUTPUT>
where
    OUTPUT: Clone + Send + Sync + 'static,
{
    type Output = OutputTree<OUTPUT>;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            OutputTree::Vec { vec, index: _ } => &vec[index],
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be indexed"),
        }
    }
}

impl<OUTPUT> std::ops::IndexMut<usize> for OutputTree<OUTPUT>
where
    OUTPUT: Clone + Send + Sync + 'static,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            OutputTree::Vec { vec, index: _ } => &mut vec[index],
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be indexed"),
        }
    }
}

impl<OUTPUT> OutputTree<OUTPUT>
where
    OUTPUT: Clone + Send + Sync + 'static,
{
    pub fn new_reef(output: Output<OUTPUT>) -> Self {
        OutputTree::Reef(Arc::new(Mutex::new(output)))
    }

    pub fn new_empty_vec() -> Self {
        OutputTree::Vec {
            vec: Vec::new(),
            index: Mutex::new(0),
        }
    }

    pub fn new_vec(vec: Vec<OutputTree<OUTPUT>>) -> Self {
        OutputTree::Vec {
            vec,
            index: Mutex::new(0),
        }
    }

    pub fn push(&mut self, output: OutputTree<OUTPUT>) {
        match self {
            OutputTree::Vec { vec, index: _ } => {
                if let OutputTree::Vec { vec, index: _ } = &output {
                    if vec.len() == 0 {
                        panic!("0 length OutputTree::Vec cannot be pushed");
                    }
                }
                vec.push(output);
            }
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be pushed"),
        }
    }

    pub fn insert(&mut self, index: usize, output: OutputTree<OUTPUT>) {
        match self {
            OutputTree::Vec { vec, index: _ } => {
                if vec.len() == 0 {
                    panic!("0 length OutputTree::Vec cannot be inserted");
                }
                vec.insert(index, output);
            }
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be inserted"),
        }
    }

    pub fn remove(&mut self, index: usize) -> OutputTree<OUTPUT> {
        match self {
            OutputTree::Vec { vec, index: _ } => vec.remove(index),
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be removed"),
        }
    }

    pub async fn loop_search(&self) -> &Arc<Mutex<Output<OUTPUT>>> {
        match self {
            OutputTree::Vec { vec, index } => {
                let len = vec.len();
                let output = &vec[*index.lock().await % len];
                *index.lock().await += 1;
                *index.lock().await %= len;
                Box::pin(output.loop_search()).await
            }
            OutputTree::Reef(socket) => socket,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            OutputTree::Vec { vec, index: _ } => {
                let mut len = 0;
                for output in vec {
                    len += output.size();
                }
                len
            }
            OutputTree::Reef(_) => 1,
        }
    }

    pub async fn get_from_id(&self, id: &SocketId) -> Result<&Arc<Mutex<Output<OUTPUT>>>, ()> {
        match self {
            OutputTree::Vec { vec, index: _ } => {
                for output in vec {
                    if let Ok(socket) = Box::pin(output.get_from_id(id)).await {
                        return Ok(socket);
                    }
                }
                Err(())
            }
            OutputTree::Reef(socket) => {
                if socket.lock().await.get_id() == id {
                    Ok(socket)
                } else {
                    Err(())
                }
            }
        }
    }

    pub async fn send_delete_cache(&self) {
        match self {
            OutputTree::Vec { vec, index: _ } => {
                for output in vec {
                    Box::pin(output.send_delete_cache()).await;
                }
            }
            OutputTree::Reef(socket) => {
                socket.lock().await.send_delete_cache().await;
            }
        }
    }
}
