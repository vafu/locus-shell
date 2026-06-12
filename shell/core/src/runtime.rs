use std::{future::Future, pin::Pin};

use providers::{TaskSpawner, TaskSpawnerAlreadyInstalled};
use tokio::task::JoinHandle;

/// Installs the default Tokio runtime used for provider subscription tasks.
pub fn install_provider_runtime() -> Result<(), TaskSpawnerAlreadyInstalled> {
    if providers::has_task_spawner() {
        return Err(TaskSpawnerAlreadyInstalled);
    }

    providers::install_task_spawner(TokioProviderSpawner::new())
}

pub(crate) fn ensure_provider_runtime() {
    let _ = install_provider_runtime();
}

#[derive(Debug)]
struct TokioProviderSpawner {
    runtime: tokio::runtime::Runtime,
}

impl TokioProviderSpawner {
    fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("shell-provider-runtime")
            .build()
            .expect("failed to initialize shell provider Tokio runtime");

        Self { runtime }
    }
}

impl TaskSpawner for TokioProviderSpawner {
    fn spawn_boxed(
        &self,
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    ) -> JoinHandle<()> {
        self.runtime.spawn(future)
    }
}
