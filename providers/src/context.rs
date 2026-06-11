use crate::CancellationToken;

/// Runtime context passed into a provider.
#[derive(Clone, Debug, Default)]
pub struct ProviderContext {
    cancellation: CancellationToken,
}

impl ProviderContext {
    /// Creates a context backed by the provided cancellation token.
    pub fn new(cancellation: CancellationToken) -> Self {
        Self { cancellation }
    }

    /// Returns the cancellation token shared with the subscription owner.
    pub fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }

    /// Returns whether the provider should stop producing values.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
}
