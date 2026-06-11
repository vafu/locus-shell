//! Backend-neutral contracts for asynchronous value providers.
//!
//! Providers expose typed value streams without depending on GTK, Relm4,
//! D-Bus, or any shell widget policy. Consumers own task spawning and keep
//! returned [`Subscription`] handles alive for as long as updates are wanted.

mod cancellation;
mod combine;
mod context;
mod error;
mod map;
mod provider;
mod sender;
mod subscription;

#[cfg(test)]
mod test;

pub use cancellation::CancellationToken;
pub use combine::{CombineLatestError, CombineLatestProvider};
pub use context::ProviderContext;
pub use error::ProviderError;
pub use map::{MapProvider, ProviderExt};
pub use provider::{Provider, provider_for, run_provider};
pub use sender::ProviderSender;
pub use subscription::{Subscription, SubscriptionGroup};
