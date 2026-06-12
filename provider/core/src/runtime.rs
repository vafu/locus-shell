use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
};

use tokio::task::JoinHandle;

static TASK_SPAWNER: OnceLock<Arc<dyn TaskSpawner>> = OnceLock::new();

/// Spawns provider tasks for subscriptions.
pub trait TaskSpawner: Send + Sync + 'static {
    /// Spawns a boxed provider task and returns the runtime task handle.
    fn spawn_boxed(
        &self,
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    ) -> JoinHandle<()>;
}

/// Returned when a provider task spawner has already been installed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskSpawnerAlreadyInstalled;

/// Installs the process-wide provider task spawner.
///
/// Framework code should call this during process setup. `providers` keeps the
/// subscription lifecycle contracts, but it does not own a runtime.
pub fn install_task_spawner(spawner: impl TaskSpawner) -> Result<(), TaskSpawnerAlreadyInstalled> {
    TASK_SPAWNER
        .set(Arc::new(spawner))
        .map_err(|_| TaskSpawnerAlreadyInstalled)
}

/// Returns whether a task spawner has already been installed.
pub fn has_task_spawner() -> bool {
    TASK_SPAWNER.get().is_some()
}

/// Spawns a provider task on the installed framework task spawner.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) -> JoinHandle<()> {
    TASK_SPAWNER
        .get()
        .expect("provider task spawner is not installed; initialize ShellApp or call providers::install_task_spawner first")
        .spawn_boxed(Box::pin(future))
}
