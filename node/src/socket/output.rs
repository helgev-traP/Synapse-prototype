use core::panic;
use std::{any::TypeId, collections::HashMap, sync::{Arc, Weak}};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::{
    channel::{NodeOrder, NodeResponse, OutputChannel},
    err::{NodeConnectError, NodeConnectionCheckError, NodeDisconnectError, NodeSendResponseError},
    node_core::NodeCore,
    types::{NodeId, SharedAny, SocketId, TryRecvResult},
};

// inner data of Output

struct OutputInner<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    // identifier
    id: SocketId,
    name: String,

    // call node's method to get data and pick up data
    pickup: Box<dyn Fn(&OUTPUT) -> SharedAny + Send + Sync>,
    // main body of node
    node: Arc<NodeCore<OUTPUT, MEMORY>>,

    // downstream sockets
    downstream: HashMap<SocketId, Weak<()>>, // todo: change to OutputSocket
}

// wrap OutputInner in Arc
// to share OutputInner between threads
// this is alternative of: impl<OUTPUT, MEMORY> Arc<Output<OUTPUT, MEMORY>> { ... }

pub struct Output<OUTPUT, MEMORY>(Arc<OutputInner<OUTPUT, MEMORY>>)
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static;

impl<OUTPUT, MEMORY> Output<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    pub fn new(
        name: String,
        pickup: Box<dyn Fn(&OUTPUT) -> SharedAny + Send + Sync>,
        main_body_of_node: Arc<NodeCore<OUTPUT, MEMORY>>,
    ) -> Self {
        Output(Arc::new(OutputInner {
            id: SocketId::new(),
            name,
            pickup,
            node: main_body_of_node,
            downstream: HashMap::new(),
        }))
    }

    pub fn get_id(&self) -> &SocketId {
        &self.0.id
    }

    pub async fn connect(
        &mut self,
        mut channel: OutputChannel,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeConnectError> {
        todo!()
    }

    pub async fn disconnect(
        &mut self,
        downstream_socket_id: &SocketId,
    ) -> Result<(), NodeDisconnectError> {
        todo!()
    }

    // 制作メモ
    // チャンネルの導通を確認するための物
    // 間違えて不適切な組のチャンネルをdropしないように確認できるようにする
    pub async fn send_connection_checker(
        &mut self,
        token: Uuid,
        socket_id: &SocketId,
    ) -> Result<(), NodeConnectionCheckError> {
        todo!()
    }

    pub async fn get_destinations(&self) -> Option<(&NodeId, &SocketId)> {
        todo!()
    }

    pub async fn send_delete_cache(&mut self) {
        todo!()
    }
}

impl<OUTPUT, MEMORY> Clone for Output<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Output(Arc::clone(&self.0))
    }
}

pub trait OutputCommon: Send + Sync + 'static
{
    // called by downstream socket
    fn call(&self) -> SharedAny;
    fn get_id(&self) -> &SocketId;
    fn get_downstream_ids(&self) -> Vec<SocketId>;
}

impl<OUTPUT, MEMORY> OutputCommon for Output<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    fn call(&self) -> SharedAny {
        todo!()
    }

    fn get_id(&self) -> &SocketId {
        &self.0.id
    }
}

// todo: indexを削除
pub enum OutputTree<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    Vec {
        vec: Vec<OutputTree<OUTPUT, MEMORY>>,
        index: Mutex<usize>,
    },
    Reef(Arc<Mutex<Output<OUTPUT, MEMORY>>>),
}

impl<OUTPUT, MEMORY> std::ops::Index<usize> for OutputTree<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    type Output = OutputTree<OUTPUT, MEMORY>;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            OutputTree::Vec { vec, index: _ } => &vec[index],
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be indexed"),
        }
    }
}

impl<OUTPUT, MEMORY> std::ops::IndexMut<usize> for OutputTree<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            OutputTree::Vec { vec, index: _ } => &mut vec[index],
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be indexed"),
        }
    }
}

impl<OUTPUT, MEMORY> OutputTree<OUTPUT, MEMORY>
where
    OUTPUT: Clone + Send + Sync + 'static,
    MEMORY: Send + Sync + 'static,
{
    pub fn new_reef(output: Output<OUTPUT, MEMORY>) -> Self {
        OutputTree::Reef(Arc::new(Mutex::new(output)))
    }

    pub fn new_empty_vec() -> Self {
        OutputTree::Vec {
            vec: Vec::new(),
            index: Mutex::new(0),
        }
    }

    pub fn new_vec(vec: Vec<OutputTree<OUTPUT, MEMORY>>) -> Self {
        OutputTree::Vec {
            vec,
            index: Mutex::new(0),
        }
    }

    pub fn push(&mut self, output: OutputTree<OUTPUT, MEMORY>) {
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

    pub fn insert(&mut self, index: usize, output: OutputTree<OUTPUT, MEMORY>) {
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

    pub fn remove(&mut self, index: usize) -> OutputTree<OUTPUT, MEMORY> {
        match self {
            OutputTree::Vec { vec, index: _ } => vec.remove(index),
            OutputTree::Reef(_) => panic!("OutputTree::Reef cannot be removed"),
        }
    }

    pub async fn loop_search(&self) -> &Arc<Mutex<Output<OUTPUT, MEMORY>>> {
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

    pub async fn get_from_id(&self, id: &SocketId) -> Result<&Arc<Mutex<Output<OUTPUT, MEMORY>>>, ()> {
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
