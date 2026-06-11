use crate::{CancellationToken, ProviderContext};

/// Owns cancellation for a running provider subscription.
///
/// Dropping the subscription requests cancellation. Provider futures are still
/// responsible for observing the context and exiting.
#[derive(Debug, Default)]
pub struct Subscription {
    cancellation: CancellationToken,
}

impl Subscription {
    /// Creates an active subscription handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a context that shares this subscription's cancellation state.
    pub fn context(&self) -> ProviderContext {
        ProviderContext::new(self.cancellation.clone())
    }

    /// Requests cancellation for the associated provider.
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.cancel();
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

    /// Requests cancellation for every subscription in the group.
    pub fn cancel(&self) {
        for subscription in &self.subscriptions {
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
