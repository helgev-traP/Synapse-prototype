use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::mpsc,
};

use node::{field::NodeField, framework::NodeFramework, node_core::NodeCoreCommon};

use crate::message::MessageToBackend;

pub struct ProjectManager {
    node_field: NodeField,
    plugins: HashMap<TypeId, Box<dyn NodeFramework>>,

    // com
    ch_tx: mpsc::Sender<()>, // TODO: define the type
    ch_rx: mpsc::Receiver<MessageToBackend>,
}

impl ProjectManager {
    pub fn new(ch_tx: mpsc::Sender<()>, ch_rx: mpsc::Receiver<MessageToBackend>) -> Self {
        ProjectManager {
            node_field: NodeField::new(""),
            plugins: HashMap::new(),
            ch_tx,
            ch_rx,
        }
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn NodeFramework>) {
        let plugin_id = (*plugin).type_id();
        if self.plugins.contains_key(&plugin_id) {
            println!("ProjectManager: plugin {:?} already exists", plugin_id);
            return;
        }
        println!(
            "ProjectManager: add plugin {:?} {}",
            plugin_id,
            plugin.name()
        );
        self.plugins.insert(plugin_id, plugin);
    }

    pub async fn run(&mut self) {
        loop {
            let msg = match self.ch_rx.recv() {
                Ok(msg) => msg,
                Err(e) => {
                    println!("ProjectManager: recv error: {}", e);
                    break;
                }
            };

            match msg {
                MessageToBackend::Shutdown => {
                    if self.node_field.field_is_playing() {
                        self.node_field.field_stop().await;
                    }
                    break;
                }
                MessageToBackend::AllPlugins(plugin_id) => {
                    if let Some(plugin) = self.plugins.get(&plugin_id) {
                        self.node_field.add_node(plugin.build().await);
                    }
                }
                MessageToBackend::FieldReq(field_req) => match field_req {
                    crate::message::field::FieldReq::Id => {
                        let id = self.node_field.get_id();
                        // TODO: send id to frontend
                        todo!()
                    }
                    crate::message::field::FieldReq::Name => {
                        let name = self.node_field.get_name().await.clone();
                        // TODO: send name to frontend
                        todo!()
                    }
                    crate::message::field::FieldReq::AllNodes(vec) => todo!(),
                    crate::message::field::FieldReq::CacheDepth { node } => todo!(),
                },
                MessageToBackend::FieldOp(field_op) => match field_op {
                    crate::message::field::FieldOp::AddNode(plugin_id) => {
                        let Some(plugin) = self.plugins.get(&plugin_id) else {
                            todo!()
                        };

                        self.node_field.add_node(plugin.build().await);

                        // todo
                    }
                    crate::message::field::FieldOp::RemoveNode(node_id) => {
                        self.node_field.remove_node(&node_id);
                    }
                    crate::message::field::FieldOp::Connect {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_field
                            .node_connect(
                                upstream_node,
                                upstream_socket,
                                downstream_node,
                                downstream_socket,
                            )
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeConnectError::NodeIdNotFound => todo!(),
                                node::err::NodeConnectError::SocketIdNotFound => todo!(),
                                node::err::NodeConnectError::TypeRejected => todo!(),
                                node::err::NodeConnectError::InputNotEmpty => todo!(),
                            },
                        }
                    }
                    crate::message::field::FieldOp::ConservativeConnect {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_field
                            .node_conservative_connect(
                                upstream_node,
                                upstream_socket,
                                downstream_node,
                                downstream_socket,
                            )
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeConnectError::NodeIdNotFound => todo!(),
                                node::err::NodeConnectError::SocketIdNotFound => todo!(),
                                node::err::NodeConnectError::TypeRejected => todo!(),
                                node::err::NodeConnectError::InputNotEmpty => todo!(),
                            },
                        }
                    }
                    crate::message::field::FieldOp::Disconnect {
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_field
                            .node_disconnect(downstream_node, downstream_socket)
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeDisconnectError::NotConnected => todo!(),
                                node::err::NodeDisconnectError::NodeIdNotFound => todo!(),
                                node::err::NodeDisconnectError::SocketIdNotFound => todo!(),
                            },
                        }
                    }
                    crate::message::field::FieldOp::ConservativeDisconnect {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_field
                            .node_conservative_disconnect(
                                upstream_node,
                                upstream_socket,
                                downstream_node,
                                downstream_socket,
                            )
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeDisconnectError::NotConnected => todo!(),
                                node::err::NodeDisconnectError::NodeIdNotFound => todo!(),
                                node::err::NodeDisconnectError::SocketIdNotFound => todo!(),
                            },
                        }
                    }
                    crate::message::field::FieldOp::CheckConnection {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_field
                            .check_connection(
                                upstream_node,
                                upstream_socket,
                                downstream_node,
                                downstream_socket,
                            )
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeConnectionCheckError::NodeIdNotFound => todo!(),
                                node::err::NodeConnectionCheckError::SocketIdNotFound => todo!(),
                                node::err::NodeConnectionCheckError::ChannelClosed => todo!(),
                                node::err::NodeConnectionCheckError::NotConnected => todo!(),
                            },
                        }
                    }
                    crate::message::field::FieldOp::DisconnectAllFromOutput { node, socket } => {
                        match self
                            .node_field
                            .node_disconnect_all_from_output(node, socket)
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeDisconnectError::NotConnected => todo!(),
                                node::err::NodeDisconnectError::NodeIdNotFound => todo!(),
                                node::err::NodeDisconnectError::SocketIdNotFound => todo!(),
                            },
                        }
                    }
                    crate::message::field::FieldOp::DisconnectAll { node } => {
                        match self.node_field.node_disconnect_all(node).await {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeDisconnectError::NotConnected => todo!(),
                                node::err::NodeDisconnectError::NodeIdNotFound => todo!(),
                                node::err::NodeDisconnectError::SocketIdNotFound => todo!(),
                            },
                        }
                    }
                    crate::message::field::FieldOp::UpdateInputDefault {
                        node,
                        socket,
                        default,
                    } => {
                        if let Err(e) = self
                            .node_field
                            .node_update_input_default(node, socket, default)
                            .await
                        {
                            match e {
                                node::err::UpdateInputDefaultError::NodeIdNotFound(any) => todo!(),
                                node::err::UpdateInputDefaultError::SocketIdNotFound(any) => {
                                    todo!()
                                }
                                node::err::UpdateInputDefaultError::TypeRejected(any) => todo!(),
                                node::err::UpdateInputDefaultError::DefaultValueNotEnabled(any) => {
                                    todo!()
                                }
                            }
                        }
                    }
                    crate::message::field::FieldOp::CacheClear(node_id) => todo!(),
                    crate::message::field::FieldOp::CacheSetDepth { node, depth } => todo!(),
                    crate::message::field::FieldOp::CacheClearAll => todo!(),
                    crate::message::field::FieldOp::CacheSetDepthAll(depth) => todo!(),
                    crate::message::field::FieldOp::Call { node, frame } => {
                        match self.node_field.field_call(frame, node).await {
                            Ok(_) => {
                                // todo
                            }
                            Err(_) => todo!(),
                        }
                    }
                    crate::message::field::FieldOp::Play { node, frame } => {
                        match self.node_field.field_play(frame, node).await {
                            Ok(_) => todo!(),
                            Err(_) => todo!(),
                        }
                    }
                    crate::message::field::FieldOp::Stop => {
                        self.node_field.field_stop().await;
                    }
                },
            }
        }
    }
}
