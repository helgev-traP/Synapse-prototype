use std::sync::Arc;

use main::manager::ProjectManager;
use matcha::{
    app::App,
    component::{Component, ComponentAccess},
    ui::Dom,
};

const GUI_THREAD: usize = 2;

fn main() {
    let (message_to_backend_tx, message_to_backend_rx) = std::sync::mpsc::channel();
    let (message_to_frontend_tx, message_to_frontend_rx) = std::sync::mpsc::channel();

    // build backend manager / node field
    let mut manager = ProjectManager::new(message_to_frontend_tx, message_to_backend_rx);
    for plugin in plugin::give_all_plugins() {
        manager.add_plugin(plugin);
    }

    // build frontend
    let mut app = App::new(Component::new(Some(""), (), update::<()>, view))
        .communicate(message_to_backend_tx, message_to_frontend_rx);

    // detect the number of cores
    let num_cores = std::thread::available_parallelism().unwrap();

    if num_cores.get() < GUI_THREAD * 2 {
        println!("cpu has less than {} cores is not supported now.", GUI_THREAD * 2);
    }

    let backend_thread = num_cores.get() - GUI_THREAD;
    let frontend_thread = GUI_THREAD;

    // build backend / frontend thread pool
    // tokio runtime with {backend_thread} threads.
    let backend_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(backend_thread)
        .enable_all()
        .build()
        .unwrap();

    // rayon thread pool with {frontend_thread} threads.
    let frontend_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(frontend_thread)
        .build()
        .unwrap();

    // start backend with tokio runtime
    println!("start backend with {} threads.", backend_thread);
    backend_rt.spawn(async move {
        manager.run().await;
    });

    // start frontend with rayon thread pool
    println!("start frontend with {} threads.", frontend_thread);
    frontend_pool.spawn(move|| {
        app.run();
    });
}

fn view<R>(model: &()) -> Box<dyn Dom<R>> {
    todo!()
}

fn update<R>(state: &ComponentAccess<()>, message: ()) {
    todo!()
}
