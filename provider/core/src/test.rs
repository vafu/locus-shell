use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, Once,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use futures::{StreamExt as FuturesStreamExt, stream};
use tokio::task::JoinHandle;
use tokio_stream::Stream;

use super::{
    CancellationToken, CombineLatestError, Provider, ProviderError, ProviderExt, Subscription,
    SubscriptionGroup, TaskSpawner, combine_latest2, combine_latest2_stream, install_task_spawner,
    provider_for, run_provider, spawn,
};

#[derive(Debug)]
struct TestTaskSpawner {
    runtime: tokio::runtime::Runtime,
}

impl TestTaskSpawner {
    fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("providers-test-runtime")
            .build()
            .expect("provider test runtime");

        Self { runtime }
    }
}

impl TaskSpawner for TestTaskSpawner {
    fn spawn_boxed(
        &self,
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    ) -> JoinHandle<()> {
        self.runtime.spawn(future)
    }
}

fn ensure_test_spawner() {
    static INSTALL: Once = Once::new();

    INSTALL.call_once(|| {
        install_task_spawner(TestTaskSpawner::new()).expect("install provider test spawner");
    });
}

#[test]
fn subscription_spawn_token_tracks_cancellation() {
    ensure_test_spawner();

    let (sender, receiver) = std::sync::mpsc::channel();
    let mut subscription = Subscription::spawn(|cancellation| async move {
        sender.send(cancellation).expect("send cancellation token");
        futures::future::pending::<()>().await;
    });
    let cancellation = receiver.recv().expect("cancellation token");

    assert!(!cancellation.is_cancelled());
    subscription.cancel();
    assert!(cancellation.is_cancelled());
}

#[test]
fn dropping_subscription_cancels_token() {
    let (subscription, cancellation) = subscription_with_cancellation();

    drop(subscription);

    assert!(cancellation.is_cancelled());
}

#[test]
fn cancellation_token_can_be_awaited() {
    let token = CancellationToken::new();

    token.cancel();
    futures::executor::block_on(token.cancelled());
}

#[test]
fn dropping_subscription_aborts_registered_task() {
    ensure_test_spawner();

    let (sender, receiver) = std::sync::mpsc::channel();
    let subscription = Subscription::spawn(|_context| async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        sender.send("late").expect("send runtime result");
    });

    drop(subscription);

    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
}

#[test]
fn subscription_group_cancels_all_contexts() {
    let mut subscriptions = SubscriptionGroup::new();
    let first = subscription_with_cancellation();
    let second = subscription_with_cancellation();
    let first_cancellation = first.1;
    let second_cancellation = second.1;

    subscriptions.push(first.0);
    subscriptions.push(second.0);

    assert_eq!(subscriptions.len(), 2);
    assert!(!subscriptions.is_empty());
    assert!(!first_cancellation.is_cancelled());
    assert!(!second_cancellation.is_cancelled());

    subscriptions.cancel();

    assert!(subscriptions.is_empty());
    assert!(first_cancellation.is_cancelled());
    assert!(second_cancellation.is_cancelled());
}

#[test]
fn dropping_subscription_group_cancels_all_contexts() {
    let (first_cancellation, second_cancellation) = {
        let mut subscriptions = SubscriptionGroup::new();
        let first = subscription_with_cancellation();
        let second = subscription_with_cancellation();
        let first_cancellation = first.1;
        let second_cancellation = second.1;

        subscriptions.push(first.0);
        subscriptions.push(second.0);

        (first_cancellation, second_cancellation)
    };

    assert!(first_cancellation.is_cancelled());
    assert!(second_cancellation.is_cancelled());
}

#[test]
fn provider_runner_forwards_result_items() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let provider = tokio_stream::iter([
        Ok::<_, ProviderError>(1_u32),
        Err(ProviderError::new("watch failed")),
        Ok(2),
    ]);

    futures::executor::block_on(run_provider(
        provider,
        CancellationToken::new(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert_eq!(
        *values.lock().expect("values lock"),
        [Ok(1), Err(ProviderError::new("watch failed")), Ok(2)]
    );
}

#[test]
fn provider_for_preserves_matching_provider() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let provider = provider_for::<String, _>(tokio_stream::iter([Ok::<_, Infallible>(
        "ready".to_owned(),
    )]));

    futures::executor::block_on(run_provider(
        provider,
        CancellationToken::new(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert_eq!(
        *values.lock().expect("values lock"),
        [Ok::<_, Infallible>("ready".to_owned())]
    );
}

#[test]
fn spawn_runs_future_on_provider_runtime() {
    ensure_test_spawner();

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
fn stream_providers_can_use_tokio_stream_ext_directly() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let provider = tokio_stream::iter([
        Ok::<_, Infallible>("ready".to_owned()),
        Ok("steady".to_owned()),
    ]);
    let provider = FuturesStreamExt::map(provider, |value: Result<String, Infallible>| {
        value.map(|value| value.len())
    });

    futures::executor::block_on(run_provider(
        provider,
        CancellationToken::new(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert_eq!(
        *values.lock().expect("values lock"),
        [Ok::<_, Infallible>(5), Ok(6)]
    );
}

#[test]
fn combine_latest2_stream_emits_after_both_sources_have_values() {
    let left = tokio_stream::iter([Ok::<_, ProviderError>(1), Ok(2)]);
    let right = tokio_stream::iter([
        Ok::<_, ProviderError>("alpha".to_owned()),
        Ok("beta".to_owned()),
    ]);
    let combined = combine_latest2_stream(left, right);

    let values: Vec<_> = futures::executor::block_on(combined.collect());

    assert_eq!(
        values,
        [
            Ok((1, "alpha".to_owned())),
            Ok((2, "alpha".to_owned())),
            Ok((2, "beta".to_owned()))
        ]
    );
}

#[test]
fn combine_latest2_stream_forwards_errors_without_clearing_latest_values() {
    let left = tokio_stream::iter([
        Ok::<_, ProviderError>(1),
        Err(ProviderError::new("left failed")),
        Ok(2),
    ]);
    let right = tokio_stream::iter([Ok::<_, ProviderError>(10)]);
    let combined = combine_latest2_stream(left, right);

    let values: Vec<_> = futures::executor::block_on(combined.collect());

    assert_eq!(
        values,
        [
            Ok((1, 10)),
            Err(CombineLatestError::Left(ProviderError::new("left failed"))),
            Ok((2, 10)),
        ]
    );
}

#[test]
fn combine_latest2_stream_ends_if_a_source_finishes_before_first_value() {
    let left = tokio_stream::iter([]);
    let right = tokio_stream::iter([Ok::<_, ProviderError>(10)]);
    let combined = combine_latest2_stream(left, right);

    let values: Vec<Result<(u32, u32), CombineLatestError<ProviderError, ProviderError>>> =
        futures::executor::block_on(combined.collect());

    assert!(values.is_empty());
}

#[test]
fn combine_latest2_provider_combines_provider_streams() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let captured = values.clone();
    let left = tokio_stream::iter([Ok::<_, Infallible>(1), Ok(2)]);
    let right = tokio_stream::iter([Ok::<_, Infallible>(3)]);
    let provider = combine_latest2(left, right);

    futures::executor::block_on(run_provider(
        provider,
        CancellationToken::new(),
        move |value| {
            captured.lock().expect("values lock").push(value);
        },
    ));

    assert_eq!(
        *values.lock().expect("values lock"),
        [
            Ok::<_, CombineLatestError<Infallible, Infallible>>((1, 3)),
            Ok((2, 3))
        ]
    );
}

#[test]
fn shared_provider_reuses_active_upstream_and_restarts_later() {
    ensure_test_spawner();

    #[derive(Clone)]
    struct CountedProvider {
        runs: Arc<AtomicUsize>,
    }

    impl Provider<u32> for CountedProvider {
        type Error = ProviderError;
        type Stream = Pin<Box<dyn Stream<Item = Result<u32, ProviderError>> + Send>>;

        fn stream(self, _cancellation: CancellationToken) -> Self::Stream {
            self.runs.fetch_add(1, Ordering::SeqCst);
            Box::pin(FuturesStreamExt::chain(
                stream::iter([Ok(7)]),
                stream::pending(),
            ))
        }
    }

    let runs = Arc::new(AtomicUsize::new(0));
    let shared = CountedProvider { runs: runs.clone() }.shared();

    let mut first = shared.clone().stream(CancellationToken::new());
    assert_eq!(
        futures::executor::block_on(FuturesStreamExt::next(&mut first)),
        Some(Ok(7))
    );
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    let mut second = shared.clone().stream(CancellationToken::new());
    assert_eq!(
        futures::executor::block_on(FuturesStreamExt::next(&mut second)),
        Some(Ok(7))
    );
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    drop(first);
    drop(second);

    let mut third = shared.stream(CancellationToken::new());
    assert_eq!(
        futures::executor::block_on(FuturesStreamExt::next(&mut third)),
        Some(Ok(7))
    );
    assert_eq!(runs.load(Ordering::SeqCst), 2);
}

fn subscription_with_cancellation() -> (Subscription, CancellationToken) {
    ensure_test_spawner();

    let (sender, receiver) = std::sync::mpsc::channel();
    let subscription = Subscription::spawn(|cancellation| async move {
        sender.send(cancellation).expect("send cancellation token");
        futures::future::pending::<()>().await;
    });
    let cancellation = receiver.recv().expect("cancellation token");

    (subscription, cancellation)
}
