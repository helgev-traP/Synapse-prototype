use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::mpsc,
};

use node::{node::NodeCommon, node_controller::NodeController, plugin::Plugin};

use crate::message::MessageToBackend;

pub struct ProjectManager {
    node_controller: NodeController,
    plugins: HashMap<TypeId, Box<dyn Plugin>>,

    // com
    ch_tx: mpsc::Sender<()>, // TODO: define the type
    ch_rx: mpsc::Receiver<MessageToBackend>,
}

impl ProjectManager {
    pub fn new(ch_tx: mpsc::Sender<()>, ch_rx: mpsc::Receiver<MessageToBackend>) -> Self {
        ProjectManager {
            node_controller: NodeController::new(""),
            plugins: HashMap::new(),
            ch_tx,
            ch_rx,
        }
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn Plugin>) {
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
                    if self.node_controller.controller_is_playing() {
                        self.node_controller.controller_stop().await;
                    }
                    break;
                }
                MessageToBackend::AllPlugins(plugin_id) => {
                    if let Some(plugin) = self.plugins.get(&plugin_id) {
                        self.node_controller.add_node(plugin.build());
                    }
                }
                MessageToBackend::ControllerReq(controller_req) => match controller_req {
                    crate::message::controller::ControllerReq::Id => {
                        let id = self.node_controller.get_id();
                        // TODO: send id to frontend
                        todo!()
                    }
                    crate::message::controller::ControllerReq::Name => {
                        let name = self.node_controller.get_name().await.clone();
                        // TODO: send name to frontend
                        todo!()
                    }
                    crate::message::controller::ControllerReq::AllNodes(vec) => todo!(),
                    crate::message::controller::ControllerReq::CacheDepth { node } => todo!(),
                },
                MessageToBackend::ControllerOp(controller_op) => match controller_op {
                    crate::message::controller::ControllerOp::AddNode(plugin_id) => {
                        let Some(plugin) = self.plugins.get(&plugin_id) else {
                            todo!()
                        };

                        self.node_controller.add_node(plugin.build());

                        // todo
                    }
                    crate::message::controller::ControllerOp::RemoveNode(node_id) => {
                        self.node_controller.remove_node(node_id).await.unwrap();
                    }
                    crate::message::controller::ControllerOp::Connect {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_controller
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
                    crate::message::controller::ControllerOp::ConservativeConnect {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_controller
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
                    crate::message::controller::ControllerOp::Disconnect {
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_controller
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
                    crate::message::controller::ControllerOp::ConservativeDisconnect {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_controller
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
                    crate::message::controller::ControllerOp::CheckConnection {
                        upstream_node,
                        upstream_socket,
                        downstream_node,
                        downstream_socket,
                    } => {
                        match self
                            .node_controller
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
                    crate::message::controller::ControllerOp::DisconnectAllFromOutput {
                        node,
                        socket,
                    } => {
                        match self
                            .node_controller
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
                    crate::message::controller::ControllerOp::DisconnectAll { node } => {
                        match self.node_controller.node_disconnect_all(node).await {
                            Ok(_) => (),
                            Err(e) => match e {
                                node::err::NodeDisconnectError::NotConnected => todo!(),
                                node::err::NodeDisconnectError::NodeIdNotFound => todo!(),
                                node::err::NodeDisconnectError::SocketIdNotFound => todo!(),
                            },
                        }
                    }
                    crate::message::controller::ControllerOp::UpdateInputDefault {
                        node,
                        socket,
                        default,
                    } => {
                        if let Err(e) = self
                            .node_controller
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
                    crate::message::controller::ControllerOp::CacheClear(node_id) => todo!(),
                    crate::message::controller::ControllerOp::CacheSetDepth { node, depth } => {
                        todo!()
                    }
                    crate::message::controller::ControllerOp::CacheClearAll => todo!(),
                    crate::message::controller::ControllerOp::CacheSetDepthAll(depth) => todo!(),
                    crate::message::controller::ControllerOp::Call { node, frame } => {
                        match self.node_controller.controller_call(frame, node).await {
                            Ok(_) => {
                                // todo
                            }
                            Err(_) => todo!(),
                        }
                    }
                    crate::message::controller::ControllerOp::Play { node, frame } => {
                        match self.node_controller.controller_play(frame, node).await {
                            Ok(_) => todo!(),
                            Err(_) => todo!(),
                        }
                    }
                    crate::message::controller::ControllerOp::Stop => {
                        self.node_controller.controller_stop().await;
                    }
                },
            }
        }
    }
}
