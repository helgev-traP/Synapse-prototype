use std::{
    any::TypeId,
    sync::{
        mpsc::{self, TryRecvError},
        Arc, RwLock,
    },
};

// channel

pub struct ChannelCom<TX, RX> {
    pub tx: mpsc::Sender<TX>,
    pub rx: mpsc::Receiver<RX>,
}

impl<T, S> ChannelCom<T, S> {
    pub fn new(tx: mpsc::Sender<T>, rx: mpsc::Receiver<S>) -> Self {
        ChannelCom { tx, rx }
    }
}

pub enum BackendToNode {
    Shutdown,
}

pub enum NodeToBackend {
    Error,
}

pub enum FrontendToNode {}

pub enum NodeToFrontend {}

pub enum NodeOrder {
    Request { frame: i64 },
    Type,
}

#[derive(Clone)]
pub enum NodeResponse {
    Response { frame: i64 },
    ProcessFailed,
    TypeId(TypeId),
    Shared(Arc<RwLock<dyn shared::Shared>>),
}

// shared memory

pub mod shared {
    use std::any::{Any, TypeId};

    pub trait Shared {
        fn type_id(&self) -> TypeId;
        fn value(&self) -> Box<dyn Any>;
    }

    impl<T: 'static> Shared for T {
        fn type_id(&self) -> TypeId {
            TypeId::of::<T>()
        }

        fn value(&self) -> Box<dyn Any> {
            todo!()
        }
    }

    pub fn is_same_id<T: 'static, U: 'static>(_t: T, _u: U) -> bool {
        TypeId::of::<T>() == TypeId::of::<U>()
    }

    pub fn cast_to<T: 'static + Copy>(any: &dyn Any) -> Option<T> {
        match any.downcast_ref::<T>() {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}