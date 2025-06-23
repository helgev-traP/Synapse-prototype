use std::any::TypeId;

pub mod controller;

pub enum MessageToBackend {
    // manager
    AllPlugins(TypeId),
    Shutdown,
    // controller
    ControllerReq(controller::ControllerReq),
    ControllerOp(controller::ControllerOp),
}

pub enum MessageFromBackend {}
