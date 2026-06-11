use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use super::{
    Provider, ProviderContext, ProviderError, ProviderExt, ProviderSender, Subscription,
    SubscriptionGroup, provider_for, run_provider, spawn, stream_provider,
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

struct DelayedValueProvider<T> {
    delay: Duration,
    value: T,
}

impl<T> Provider<T> for DelayedValueProvider<T>
where
    T: Send + 'static,
{
    type Error = Infallible;

    async fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> Result<(), Self::Error> {
        tokio::select! {
            _ = context.cancelled() => {}
            _ = tokio::time::sleep(self.delay) => {
                sender.send(self.value);
            }
        }

        Ok(())
    }
}

struct TimedSequenceProvider<T> {
    values: Vec<(Duration, T)>,
}

impl<T> Provider<T> for TimedSequenceProvider<T>
where
    T: Send + 'static,
{
    type Error = Infallible;

    async fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> Result<(), Self::Error> {
        for (delay, value) in self.values {
            tokio::select! {
                _ = context.cancelled() => return Ok(()),
                _ = tokio::time::sleep(delay) => {
                    sender.send(value);
                }
            }
        }

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
fn cancellation_context_can_be_awaited() {
    let subscription = Subscription::new();
    let context = subscription.context();

    subscription.cancel();
    futures::executor::block_on(context.cancelled());
}

#[test]
fn dropping_subscription_aborts_registered_task() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut subscription = Subscription::new();
    let task = spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        sender.send("late").expect("send runtime result");
    });

    subscription.set_task(task);
    drop(subscription);

    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
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
fn spawn_runs_future_on_provider_runtime() {
    let (sender, receiver) = std::sync::mpsc::channel();

    let _task = spawn(async move {
        sender.send("ready").expect("send runtime result");
    });

    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("runtime result"),
        "ready"
    );
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

#[test]
fn stream_provider_forwards_stream_values() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let stream = tokio_stream::iter([
        Ok::<_, Infallible>(1_u32),
        Ok::<_, Infallible>(2_u32),
        Ok::<_, Infallible>(3_u32),
    ]);

    let result = futures::executor::block_on(run_provider(
        stream_provider(stream),
        ProviderContext::default(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert!(result.is_ok());
    assert_eq!(*values.lock().expect("values lock"), [1, 2, 3]);
}

#[test]
fn provider_switch_map_keeps_latest_downstream() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let upstream = TimedSequenceProvider {
        values: vec![(Duration::ZERO, 1_u32), (Duration::from_millis(25), 2_u32)],
    };
    let provider = upstream.switch_map(|value| DelayedValueProvider {
        delay: if value == 1 {
            Duration::from_millis(100)
        } else {
            Duration::ZERO
        },
        value,
    });

    let result = futures::executor::block_on(run_provider(
        provider,
        ProviderContext::default(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert!(result.is_ok());
    assert_eq!(*values.lock().expect("values lock"), [2]);
}

#[test]
fn shared_provider_reuses_upstream_and_replays_latest() {
    struct CountedProvider {
        runs: Arc<AtomicUsize>,
    }

    impl Provider<u32> for CountedProvider {
        type Error = Infallible;

        async fn run(
            self,
            _context: ProviderContext,
            sender: ProviderSender<u32>,
        ) -> Result<(), Self::Error> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            sender.send(7);
            Ok(())
        }
    }

    let runs = Arc::new(AtomicUsize::new(0));
    let shared = CountedProvider { runs: runs.clone() }.shared();
    let first_values = Arc::new(Mutex::new(Vec::new()));
    let first_captured = first_values.clone();

    let first = futures::executor::block_on(run_provider(
        shared.clone(),
        ProviderContext::default(),
        move |value| {
            first_captured
                .lock()
                .expect("first values lock")
                .push(value);
        },
    ));

    assert!(first.is_ok());
    assert_eq!(*first_values.lock().expect("first values lock"), [7]);
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    let second_values = Arc::new(Mutex::new(Vec::new()));
    let second_captured = second_values.clone();
    let second = futures::executor::block_on(run_provider(
        shared,
        ProviderContext::default(),
        move |value| {
            second_captured
                .lock()
                .expect("second values lock")
                .push(value);
        },
    ));

    assert!(second.is_ok());
    assert_eq!(*second_values.lock().expect("second values lock"), [7]);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}
