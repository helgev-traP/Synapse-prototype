use node;


fn main() {
    // detect the number of cores
    let num_cores = std::thread::available_parallelism().unwrap();

    if num_cores.get() < 4 {
        println!("cpu has less than 4 cores is not supported now.");
    }

    let backend_thread = num_cores.get() - 2;
    let frontend_thread = 2;

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

    // start backend

    // start frontend
}
