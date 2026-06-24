use std::{
    any::{Any, TypeId},
    collections::HashMap,
    collections::VecDeque,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::{Stream, StreamExt};
use rxrust::{
    context::Context,
    observer::Observer,
    prelude::{
        BoxedSubscriptionSend, CoreObservable, IntoBoxedSubscription, Observable as _,
        ObservableFactory as _, ObservableType, Shared, SharedSubject, Subscription,
    },
};

use super::{Observable, SourceError, SourceErrors, WatchEvent};

const MAX_SOURCE_ERRORS: usize = 20;

pub fn from_stream_result<T, S>(stream: S) -> Observable<T>
where
    T: Send + 'static,
    S: Stream<Item = Result<T, String>> + Send + 'static,
{
    Shared::<()>::lift(AbortableStreamResult { stream }).box_it()
}

struct AbortableStreamResult<S> {
    stream: S,
}

impl<T, S> ObservableType for AbortableStreamResult<S>
where
    S: Stream<Item = Result<T, String>>,
{
    type Item<'a>
        = T
    where
        Self: 'a;
    type Err = String;
}

impl<T, S, C> CoreObservable<C> for AbortableStreamResult<S>
where
    T: Send + 'static,
    S: Stream<Item = Result<T, String>> + Send + 'static,
    C: Context,
    C::Inner: Observer<T, String> + Send + 'static,
{
    type Unsub = AbortableStreamSubscription;

    fn subscribe(self, context: C) -> Self::Unsub {
        let mut observer = context.into_inner();
        let mut stream = Box::pin(self.stream);
        let handle = tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                if observer.is_closed() {
                    return;
                }
                match result {
                    Ok(value) => observer.next(value),
                    Err(error) => {
                        observer.error(error);
                        return;
                    }
                }
            }
            observer.complete();
        });

        AbortableStreamSubscription {
            handle: Some(handle),
        }
    }
}

struct AbortableStreamSubscription {
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Subscription for AbortableStreamSubscription {
    fn unsubscribe(mut self) {
        self.abort();
    }

    fn is_closed(&self) -> bool {
        self.handle
            .as_ref()
            .is_none_or(tokio::task::JoinHandle::is_finished)
    }
}

impl AbortableStreamSubscription {
    fn abort(&mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };
        handle.abort();
    }
}

impl Drop for AbortableStreamSubscription {
    fn drop(&mut self) {
        self.abort();
    }
}

pub fn shared_source<T>(
    kind: &'static str,
    path: PathBuf,
    create: impl Fn(PathBuf) -> Observable<T> + Send + Sync + 'static,
) -> Observable<T>
where
    T: Clone + Send + 'static,
{
    let key = SourceKey {
        kind,
        type_id: TypeId::of::<T>(),
        path,
    };

    let mut cache = source_cache().lock().expect("source cache lock poisoned");
    if let Some(hub) = cache
        .get(&key)
        .and_then(|value| value.downcast_ref::<Arc<ShareReplayHub<T>>>())
    {
        trace_source_lifecycle("cache hit", &source_key_label(&key));
        return share_replay_latest(hub.clone());
    }

    trace_source_lifecycle("cache miss", &source_key_label(&key));
    let label = format!("{}:{}", key.kind, key.path.display());
    let path = key.path.clone();
    let hub = Arc::new(ShareReplayHub::new(label, move || create(path.clone())));
    cache.insert(key, Box::new(hub.clone()));
    share_replay_latest(hub)
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SourceKey {
    kind: &'static str,
    type_id: TypeId,
    path: PathBuf,
}

fn source_cache() -> &'static Mutex<HashMap<SourceKey, Box<dyn Any + Send>>> {
    static SOURCE_CACHE: OnceLock<Mutex<HashMap<SourceKey, Box<dyn Any + Send>>>> = OnceLock::new();
    SOURCE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn source_key_label(key: &SourceKey) -> String {
    format!("{}:{} type={:?}", key.kind, key.path.display(), key.type_id)
}

fn share_replay_latest<T>(hub: Arc<ShareReplayHub<T>>) -> Observable<T>
where
    T: Clone + Send + 'static,
{
    Shared::<()>::lift(ShareReplayLatest { hub }).box_it()
}

#[derive(Clone)]
struct ShareReplayLatest<T> {
    hub: Arc<ShareReplayHub<T>>,
}

struct ShareReplayHub<T> {
    label: String,
    create: Box<dyn Fn() -> Observable<T> + Send + Sync>,
    state: Arc<Mutex<ShareReplayState<T>>>,
}

impl<T> ShareReplayHub<T> {
    fn new(label: String, create: impl Fn() -> Observable<T> + Send + Sync + 'static) -> Self {
        Self {
            label,
            create: Box::new(create),
            state: Arc::new(Mutex::new(ShareReplayState::default())),
        }
    }
}

struct ShareReplayState<T> {
    subject: SharedSubject<'static, T, String>,
    latest: Option<T>,
    subscribers: usize,
    connecting: bool,
    connection: Option<BoxedSubscriptionSend>,
}

impl<T> Default for ShareReplayState<T> {
    fn default() -> Self {
        Self {
            subject: Shared::subject(),
            latest: None,
            subscribers: 0,
            connecting: false,
            connection: None,
        }
    }
}

impl<T> ObservableType for ShareReplayLatest<T>
where
    T: Clone + Send + 'static,
{
    type Item<'a>
        = T
    where
        Self: 'a;
    type Err = String;
}

impl<T, C> CoreObservable<C> for ShareReplayLatest<T>
where
    T: Clone + Send + 'static,
    C: Context,
    C::Inner: Observer<T, String> + Send + 'static,
{
    type Unsub = ShareReplaySubscription<T>;

    fn subscribe(self, context: C) -> Self::Unsub {
        let mut observer = context.into_inner();
        let (latest, subject, should_connect) = {
            let mut state = self
                .hub
                .state
                .lock()
                .expect("share replay state lock poisoned");
            if state.subject.is_closed() {
                state.subject = Shared::subject();
                state.latest = None;
                state.connecting = false;
                state.connection = None;
            }
            state.subscribers += 1;
            let should_connect = state.connection.is_none() && !state.connecting;
            if should_connect {
                state.connecting = true;
            }
            trace_source_lifecycle(
                &format!("subscribe subscribers={}", state.subscribers),
                &self.hub.label,
            );
            (state.latest.clone(), state.subject.clone(), should_connect)
        };

        if let Some(latest) = latest {
            observer.next(latest);
        }

        let subject_subscription = subject.clone().subscribe_with(observer).into_boxed();
        if should_connect {
            trace_source_lifecycle("connect", &self.hub.label);
            let observer = ShareReplayObserver {
                subject,
                state: self.hub.state.clone(),
            };
            let connection: BoxedSubscriptionSend =
                (self.hub.create)().subscribe_with(observer).into_boxed();
            let mut state = self
                .hub
                .state
                .lock()
                .expect("share replay state lock poisoned");
            state.connecting = false;
            if state.subscribers == 0 {
                connection.unsubscribe();
            } else {
                state.connection = Some(connection);
            }
        }

        ShareReplaySubscription {
            label: self.hub.label.clone(),
            state: self.hub.state.clone(),
            subject_subscription: Some(subject_subscription),
        }
    }
}

struct ShareReplayObserver<T> {
    subject: SharedSubject<'static, T, String>,
    state: Arc<Mutex<ShareReplayState<T>>>,
}

impl<T> Observer<T, String> for ShareReplayObserver<T>
where
    T: Clone + Send + 'static,
{
    fn next(&mut self, value: T) {
        self.state
            .lock()
            .expect("share replay state lock poisoned")
            .latest = Some(value.clone());
        self.subject.next(value);
    }

    fn error(self, error: String) {
        if let Ok(mut state) = self.state.lock() {
            state.connecting = false;
            state.connection = None;
        }
        self.subject.error(error);
    }

    fn complete(self) {
        if let Ok(mut state) = self.state.lock() {
            state.connecting = false;
            state.connection = None;
        }
        self.subject.complete();
    }

    fn is_closed(&self) -> bool {
        self.subject.is_closed()
    }
}

struct ShareReplaySubscription<T> {
    label: String,
    state: Arc<Mutex<ShareReplayState<T>>>,
    subject_subscription: Option<BoxedSubscriptionSend>,
}

impl<T> Subscription for ShareReplaySubscription<T> {
    fn unsubscribe(mut self) {
        self.unsubscribe_inner();
    }

    fn is_closed(&self) -> bool {
        self.subject_subscription
            .as_ref()
            .is_none_or(Subscription::is_closed)
    }
}

impl<T> ShareReplaySubscription<T> {
    fn unsubscribe_inner(&mut self) {
        let Some(subscription) = self.subject_subscription.take() else {
            return;
        };

        subscription.unsubscribe();

        let mut state = self.state.lock().expect("share replay state lock poisoned");
        state.subscribers = state.subscribers.saturating_sub(1);
        trace_source_lifecycle(
            &format!("unsubscribe subscribers={}", state.subscribers),
            &self.label,
        );
        if state.subscribers == 0 {
            state.latest = None;
            if let Some(connection) = state.connection.take() {
                trace_source_lifecycle("disconnect", &self.label);
                connection.unsubscribe();
            }
        }
    }
}

impl<T> Drop for ShareReplaySubscription<T> {
    fn drop(&mut self) {
        // This is not a guard replacement: it is the refcount cleanup for this
        // custom shared source. Component-owned subscriptions are still guarded
        // by shell-macros via RxRust's `unsubscribe_when_dropped`.
        self.unsubscribe_inner();
    }
}

pub fn is_missing(error: &io::Error) -> bool {
    matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory)
}

pub fn watch_error(operation: &str, path: &Path, error: io::Error) -> String {
    format!("{operation} for {} failed: {error}", path.display())
}

pub fn log_errors<T>(
    source: &'static str,
    path: PathBuf,
    observable: Observable<T>,
) -> Observable<T>
where
    T: Send + 'static,
{
    observable
        .map_err(move |error| {
            record_source_error(source, &path, &error);
            eprintln!("[shell-core/source/{source}] {}: {error}", path.display());
            error
        })
        .box_it()
}

pub fn error_count() -> Observable<u64> {
    errors()
        .map(|errors| errors.total)
        .distinct_until_changed()
        .box_it()
}

pub fn errors() -> Observable<SourceErrors> {
    Shared::<()>::lift(SourceErrorSnapshots).box_it()
}

struct SourceErrorSnapshots;

impl ObservableType for SourceErrorSnapshots {
    type Item<'a> = SourceErrors;
    type Err = String;
}

impl<C> CoreObservable<C> for SourceErrorSnapshots
where
    C: Context,
    C::Inner: Observer<SourceErrors, String> + Send + 'static,
{
    type Unsub = BoxedSubscriptionSend;

    fn subscribe(self, context: C) -> Self::Unsub {
        let state = source_error_state();
        let mut observer = context.into_inner();
        observer.next(source_error_snapshot(state));
        state
            .subject
            .lock()
            .expect("source error subject lock poisoned")
            .clone()
            .subscribe_with(observer)
            .into_boxed()
    }
}

struct SourceErrorState {
    total: AtomicU64,
    recent: Mutex<VecDeque<SourceError>>,
    subject: Mutex<SharedSubject<'static, SourceErrors, String>>,
}

fn source_error_state() -> &'static SourceErrorState {
    static SOURCE_ERROR_STATE: OnceLock<SourceErrorState> = OnceLock::new();
    SOURCE_ERROR_STATE.get_or_init(|| SourceErrorState {
        total: AtomicU64::new(0),
        recent: Mutex::new(VecDeque::new()),
        subject: Mutex::new(Shared::subject()),
    })
}

fn record_source_error(source: &'static str, path: &Path, message: &str) {
    let state = source_error_state();
    let total = state.total.fetch_add(1, Ordering::SeqCst) + 1;
    let snapshot = {
        let mut recent = state
            .recent
            .lock()
            .expect("source error history lock poisoned");
        recent.push_front(SourceError {
            id: total,
            source,
            path: path.to_path_buf(),
            message: message.to_owned(),
        });
        recent.truncate(MAX_SOURCE_ERRORS);
        SourceErrors {
            total,
            recent: recent.iter().cloned().collect(),
        }
    };
    if let Ok(mut subject) = state.subject.lock() {
        subject.next(snapshot);
    }
}

fn source_error_snapshot(state: &SourceErrorState) -> SourceErrors {
    SourceErrors {
        total: state.total.load(Ordering::SeqCst),
        recent: state
            .recent
            .lock()
            .expect("source error history lock poisoned")
            .iter()
            .cloned()
            .collect(),
    }
}

fn trace_source_lifecycle(action: &str, label: &str) {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    if *ENABLED.get_or_init(|| std::env::var_os("SHELL_CORE_SOURCE_TRACE").is_some()) {
        if label.is_empty() {
            eprintln!("[shell-core/source] {action}");
        } else {
            eprintln!("[shell-core/source] {action} {label}");
        }
    }
}

pub enum OpenedWatch {
    Target(locusfs_watch::Watch),
    Parent(locusfs_watch::Watch),
}

impl OpenedWatch {
    pub fn into_parts(self) -> (locusfs_watch::Watch, bool) {
        match self {
            Self::Target(watch) => (watch, true),
            Self::Parent(watch) => (watch, false),
        }
    }
}

pub async fn open_target_or_parent(path: &Path) -> Result<OpenedWatch, String> {
    match locusfs_watch::Watch::open(path).await {
        Ok(watch) => Ok(OpenedWatch::Target(watch)),
        Err(error) if is_missing(&error) => {
            let ancestor = nearest_watchable_ancestor(path)
                .ok_or_else(|| format!("path has no watchable ancestor: {}", path.display()))?;
            locusfs_watch::Watch::open(&ancestor)
                .await
                .map(OpenedWatch::Parent)
                .map_err(|error| watch_error("open ancestor watch", &ancestor, error))
        }
        Err(error) => Err(watch_error("open watch", path, error)),
    }
}

pub struct WatchEvents {
    pending: VecDeque<WatchEvent>,
}

impl WatchEvents {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    pub async fn next(&mut self, watch: &mut locusfs_watch::Watch) -> io::Result<WatchEvent> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }

        let raw = watch.wait_raw_event().await?;
        let text = std::str::from_utf8(&raw).map_err(|error| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("watch event is not valid UTF-8: {error}"),
            )
        })?;

        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            self.pending.push_back(WatchEvent::parse_text(line)?);
        }

        self.pending
            .pop_front()
            .ok_or_else(|| io::Error::new(ErrorKind::UnexpectedEof, "empty watch event payload"))
    }
}

fn nearest_watchable_ancestor(path: &Path) -> Option<PathBuf> {
    let mut ancestor = path.parent();
    while let Some(path) = ancestor {
        if path.exists() {
            return Some(path.to_owned());
        }
        ancestor = path.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context as TaskContext, Poll},
    };

    use futures_util::Stream;
    use rxrust::prelude::{
        IntoBoxedSubscription as _, Observable as _, ObservableFactory as _, Observer, Shared,
        Subscription,
    };

    use super::{ShareReplayState, ShareReplaySubscription, from_stream_result};

    #[derive(Clone, Default)]
    struct CountSubscription {
        unsubscribe_count: Arc<AtomicUsize>,
    }

    impl CountSubscription {
        fn count(&self) -> usize {
            self.unsubscribe_count.load(Ordering::SeqCst)
        }
    }

    impl Subscription for CountSubscription {
        fn unsubscribe(self) {
            self.unsubscribe_count.fetch_add(1, Ordering::SeqCst);
        }

        fn is_closed(&self) -> bool {
            false
        }
    }

    #[test]
    fn share_replay_subscription_drop_unsubscribes_subject_and_upstream() {
        let subject_subscription = CountSubscription::default();
        let upstream_subscription = CountSubscription::default();
        let state = Arc::new(std::sync::Mutex::new(ShareReplayState {
            subject: Shared::subject(),
            latest: Some(1),
            subscribers: 1,
            connecting: false,
            connection: Some(upstream_subscription.clone().into_boxed()),
        }));

        let subscription = ShareReplaySubscription {
            label: "test:/source".to_owned(),
            state: state.clone(),
            subject_subscription: Some(subject_subscription.clone().into_boxed()),
        };

        drop(subscription);

        assert_eq!(subject_subscription.count(), 1);
        assert_eq!(upstream_subscription.count(), 1);

        let state = state.lock().expect("state lock poisoned");
        assert_eq!(state.subscribers, 0);
        assert!(state.latest.is_none());
        assert!(state.connection.is_none());
    }

    #[test]
    fn share_replay_subscription_drop_keeps_upstream_while_other_subscribers_remain() {
        let subject_subscription = CountSubscription::default();
        let upstream_subscription = CountSubscription::default();
        let state = Arc::new(std::sync::Mutex::new(ShareReplayState {
            subject: Shared::subject(),
            latest: Some(1),
            subscribers: 2,
            connecting: false,
            connection: Some(upstream_subscription.clone().into_boxed()),
        }));

        let subscription = ShareReplaySubscription {
            label: "test:/source".to_owned(),
            state: state.clone(),
            subject_subscription: Some(subject_subscription.clone().into_boxed()),
        };

        drop(subscription);

        assert_eq!(subject_subscription.count(), 1);
        assert_eq!(upstream_subscription.count(), 0);

        let state = state.lock().expect("state lock poisoned");
        assert_eq!(state.subscribers, 1);
        assert_eq!(state.latest, Some(1));
        assert!(state.connection.is_some());
    }

    struct PendingDropStream {
        drop_count: Arc<AtomicUsize>,
        poll_count: Arc<AtomicUsize>,
    }

    impl Stream for PendingDropStream {
        type Item = Result<(), String>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
            self.poll_count.fetch_add(1, Ordering::SeqCst);
            Poll::Pending
        }
    }

    impl Drop for PendingDropStream {
        fn drop(&mut self) {
            self.drop_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct IgnoreObserver;

    impl Observer<(), String> for IgnoreObserver {
        fn next(&mut self, _value: ()) {}

        fn error(self, _err: String) {}

        fn complete(self) {}

        fn is_closed(&self) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn from_stream_result_unsubscribe_drops_pending_stream() {
        let drop_count = Arc::new(AtomicUsize::new(0));
        let poll_count = Arc::new(AtomicUsize::new(0));
        let stream = PendingDropStream {
            drop_count: drop_count.clone(),
            poll_count,
        };

        let subscription = from_stream_result(stream).subscribe_with(IgnoreObserver);
        subscription.unsubscribe();

        for _ in 0..10 {
            if drop_count.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        assert_eq!(drop_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn rxrust_from_stream_result_unsubscribe_does_not_drop_pending_stream() {
        let drop_count = Arc::new(AtomicUsize::new(0));
        let poll_count = Arc::new(AtomicUsize::new(0));
        let stream = PendingDropStream {
            drop_count: drop_count.clone(),
            poll_count: poll_count.clone(),
        };

        let subscription = Shared::from_stream_result(stream).subscribe_with(IgnoreObserver);

        for _ in 0..10 {
            if poll_count.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(poll_count.load(Ordering::SeqCst), 1);

        subscription.unsubscribe();

        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        assert_eq!(drop_count.load(Ordering::SeqCst), 0);
    }
}
