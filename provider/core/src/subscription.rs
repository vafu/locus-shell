use std::future::Future;

use tokio::task::JoinHandle;

use crate::{CancellationToken, spawn};

/// Owns cancellation for a running provider subscription.
///
/// Dropping the subscription requests cooperative cancellation and aborts the
/// associated runtime task.
#[derive(Debug)]
pub struct Subscription {
    cancellation: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl Subscription {
    /// Creates a subscription from an existing cancellation token and task.
    pub fn from_task(cancellation: CancellationToken, task: JoinHandle<()>) -> Self {
        Self {
            cancellation,
            task: Some(task),
        }
    }

    /// Spawns a task on the installed provider task spawner and owns it as a subscription.
    pub fn spawn<F, Fut>(run: F) -> Self
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let cancellation = CancellationToken::new();
        let task = spawn(run(cancellation.clone()));

        Self::from_task(cancellation, task)
    }

    /// Returns the cancellation token owned by this subscription.
    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Stops the associated provider task.
    pub fn cancel(&mut self) {
        self.cancellation.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.cancel();
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

/// Owns multiple subscription handles and cancels them together.
#[derive(Debug, Default)]
pub struct SubscriptionGroup {
    subscriptions: Vec<Subscription>,
}

impl SubscriptionGroup {
    /// Creates an empty subscription group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a subscription to the group.
    pub fn push(&mut self, subscription: Subscription) {
        self.subscriptions.push(subscription);
    }

    /// Stops and removes every subscription in the group.
    pub fn cancel(&mut self) {
        for mut subscription in self.subscriptions.drain(..) {
            subscription.cancel();
        }
    }

    /// Returns the number of subscriptions held by the group.
    pub fn len(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns whether the group has no subscriptions.
    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty()
    }
}

impl Drop for SubscriptionGroup {
    fn drop(&mut self) {
        self.cancel();
    }
}
