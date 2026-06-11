use std::{error::Error as StdError, future::Future};

use crate::{ProviderContext, ProviderError, ProviderSender};

/// A typed asynchronous source of values.
///
/// Implementations should periodically check [`ProviderContext::is_cancelled`]
/// and return when cancellation is requested. Consumers decide where the
/// returned future runs.
pub trait Provider<T>: Send + 'static
where
    T: Send + 'static,
{
    /// Error type returned by the provider future.
    type Error: StdError + Send + Sync + 'static;

    /// Runs the provider until it completes, fails, or observes cancellation.
    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Runs a provider and forwards each produced value to `on_value`.
pub async fn run_provider<T, P, OnValue>(
    provider: P,
    context: ProviderContext,
    on_value: OnValue,
) -> Result<(), ProviderError>
where
    T: Send + 'static,
    P: Provider<T>,
    OnValue: FnMut(T) + Send + 'static,
{
    provider
        .run(context, ProviderSender::new(on_value))
        .await
        .map_err(|error| ProviderError::new(error.to_string()))
}
