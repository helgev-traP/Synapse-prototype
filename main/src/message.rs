use std::any::TypeId;

pub mod field;

pub enum MessageToBackend {
    // manager
    AllPlugins(TypeId),
    Shutdown,
    // field
    FieldReq(field::FieldReq),
    FieldOp(field::FieldOp),
}

pub enum MessageFromBackend {
}