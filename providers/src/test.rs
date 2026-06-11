use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use super::{
    Provider, ProviderContext, ProviderError, ProviderExt, ProviderSender, Subscription,
    SubscriptionGroup, provider_for, run_provider,
};

struct StaticProvider(&'static str);

impl Provider<String> for StaticProvider {
    type Error = Infallible;

    async fn run(
        self,
        _context: ProviderContext,
        sender: ProviderSender<String>,
    ) -> Result<(), Self::Error> {
        sender.send(self.0.to_owned());
        Ok(())
    }
}

struct ValueProvider<T>(T);

impl<T> Provider<T> for ValueProvider<T>
where
    T: Send + 'static,
{
    type Error = Infallible;

    async fn run(
        self,
        _context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> Result<(), Self::Error> {
        sender.send(self.0);
        Ok(())
    }
}

#[test]
fn subscription_context_tracks_cancellation() {
    let subscription = Subscription::new();
    let context = subscription.context();

    assert!(!context.is_cancelled());
    subscription.cancel();
    assert!(context.is_cancelled());
}

#[test]
fn dropping_subscription_cancels_context() {
    let context = {
        let subscription = Subscription::new();
        let context = subscription.context();

        assert!(!context.is_cancelled());
        context
    };

    assert!(context.is_cancelled());
}

#[test]
fn subscription_group_cancels_all_contexts() {
    let mut subscriptions = SubscriptionGroup::new();
    let first = Subscription::new();
    let second = Subscription::new();
    let first_context = first.context();
    let second_context = second.context();

    subscriptions.push(first);
    subscriptions.push(second);

    assert_eq!(subscriptions.len(), 2);
    assert!(!subscriptions.is_empty());
    assert!(!first_context.is_cancelled());
    assert!(!second_context.is_cancelled());

    subscriptions.cancel();

    assert!(first_context.is_cancelled());
    assert!(second_context.is_cancelled());
}

#[test]
fn dropping_subscription_group_cancels_all_contexts() {
    let (first_context, second_context) = {
        let mut subscriptions = SubscriptionGroup::new();
        let first = Subscription::new();
        let second = Subscription::new();
        let first_context = first.context();
        let second_context = second.context();

        subscriptions.push(first);
        subscriptions.push(second);

        (first_context, second_context)
    };

    assert!(first_context.is_cancelled());
    assert!(second_context.is_cancelled());
}

#[test]
fn provider_runner_forwards_values() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();

    let result = futures::executor::block_on(run_provider(
        StaticProvider("ready"),
        ProviderContext::default(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert!(result.is_ok());
    assert_eq!(*values.lock().expect("values lock"), ["ready".to_owned()]);
}

#[test]
fn provider_for_preserves_matching_provider() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let provider = provider_for::<String, _>(StaticProvider("ready"));

    let result = futures::executor::block_on(run_provider(
        provider,
        ProviderContext::default(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert!(result.is_ok());
    assert_eq!(*values.lock().expect("values lock"), ["ready".to_owned()]);
}

#[test]
fn provider_error_exposes_message() {
    let error = ProviderError::new("watch failed");

    assert_eq!(error.message(), "watch failed");
    assert_eq!(error.to_string(), "watch failed");
}

#[test]
fn provider_map_derives_values() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();

    let result = futures::executor::block_on(run_provider(
        StaticProvider("ready").map(|value| value.len()),
        ProviderContext::default(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert!(result.is_ok());
    assert_eq!(*values.lock().expect("values lock"), [5]);
}

#[test]
fn provider_combine_latest_derives_from_two_sources() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let provider =
        ValueProvider(2_u32).combine_latest(ValueProvider(40_u32), |left, right| left + right);

    let result = futures::executor::block_on(run_provider(
        provider,
        ProviderContext::default(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert!(result.is_ok());
    assert_eq!(*values.lock().expect("values lock"), [42]);
}
