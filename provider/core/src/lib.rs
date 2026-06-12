//! Backend-neutral contracts for asynchronous value providers.
//!
//! Providers expose typed value streams without depending on GTK, Relm4,
//! D-Bus, or any shell widget policy. Consumers own task spawning and keep
//! returned [`Subscription`] handles alive for as long as updates are wanted.

mod error;
mod runtime;
mod shared;
mod subscription;

#[cfg(test)]
mod test;

use std::error::Error as StdError;

use tokio_stream::{Stream, StreamExt};
pub use tokio_util::sync::CancellationToken;

pub use error::ProviderError;
pub use runtime::spawn;
pub use shared::SharedProvider;
pub use subscription::{Subscription, SubscriptionGroup};

/// A typed asynchronous source of values.
///
/// Providers are domain-level wrappers around Tokio streams. Each item carries
/// either the next value or the provider's structured error type.
pub trait Provider<T>: Send + 'static
where
    T: Send + 'static,
{
    /// Error type produced by the provider stream.
    type Error: StdError + Send + Sync + 'static;

    /// Stream of provider updates.
    type Stream: Stream<Item = Result<T, Self::Error>> + Send + 'static;

    /// Opens a stream for this provider.
    fn stream(self, cancellation: CancellationToken) -> Self::Stream;
}

impl<T, E, S> Provider<T> for S
where
    T: Send + 'static,
    E: StdError + Send + Sync + 'static,
    S: Stream<Item = Result<T, E>> + Send + 'static,
{
    type Error = E;
    type Stream = S;

    fn stream(self, _cancellation: CancellationToken) -> Self::Stream {
        self
    }
}

/// Returns `provider` only if it provides values of type `T`.
///
/// This is mainly useful for generated code that wants a focused type-checking
/// point before handing the provider to a runner.
pub fn provider_for<T, P>(provider: P) -> P
where
    T: Send + 'static,
    P: Provider<T>,
{
    provider
}

/// Runs a provider stream and forwards each produced result to `on_item`.
///
/// This keeps provider errors available to UI models as structured values,
/// for example generated `Msg::Battery(Result<f64, BatteryError>)` variants.
pub async fn run_provider<T, P, OnItem>(
    provider: P,
    cancellation: CancellationToken,
    mut on_item: OnItem,
) where
    T: Send + 'static,
    P: Provider<T>,
    OnItem: FnMut(Result<T, P::Error>) + Send + 'static,
{
    let stream = provider.stream(cancellation.clone());
    tokio::pin!(stream);

    loop {
        let value = tokio::select! {
            _ = cancellation.cancelled() => return,
            value = stream.next() => value,
        };

        let Some(value) = value else {
            return;
        };

        on_item(value);
    }
}

/// Extension methods for provider helpers that remain part of the core API.
pub trait ProviderExt<T>: Provider<T> + Sized
where
    T: Send + 'static,
{
    /// Shares one upstream provider run across active downstream subscribers.
    ///
    /// The upstream starts on the first subscriber, stops when the last
    /// subscriber drops, and restarts if a later subscriber appears.
    fn shared(self) -> SharedProvider<Self, T>
    where
        Self: Clone,
        Self: Sync,
        Self::Error: Clone,
        T: Clone + Sync,
    {
        SharedProvider::new(self)
    }
}

impl<T, P> ProviderExt<T> for P
where
    T: Send + 'static,
    P: Provider<T>,
{
}
