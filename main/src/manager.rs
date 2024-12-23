use std::sync::mpsc;

use node::field::NodeField;

use crate::message::MessageToBackend;

pub struct ProjectManager {
    node_field: NodeField,

    // com
    ch_tx: mpsc::Sender<MessageToBackend>,
    ch_rx: mpsc::Receiver<MessageToBackend>,
}

impl ProjectManager {
}