use node::types::PluginId;

pub mod field;

pub enum MessageToBackend {
    // manager
    AllPlugins(PluginId),
    // field
    FieldReq(field::FieldReq),
    FieldOp(field::FieldOp),
}

pub enum MessageFromBackend {
}