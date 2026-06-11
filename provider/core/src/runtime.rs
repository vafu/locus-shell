use std::{future::Future, sync::OnceLock};

use tokio::task::JoinHandle;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Spawns a provider task on the shared Tokio runtime.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) -> JoinHandle<()> {
    runtime().spawn(future)
}

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("providers-runtime")
            .build()
            .expect("failed to initialize provider Tokio runtime")
    })
}
