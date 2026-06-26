use std::{
    any::{Any, TypeId},
    collections::HashMap,
    collections::VecDeque,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, Weak,
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

use super::{Observable, SourceError, SourceErrors, WatchChange, WatchEvent};

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
    let path = crate::locus_path::LocusPath::new(path).into_path_buf();
    let key = SourceKey {
        kind,
        type_id: TypeId::of::<T>(),
        target: SourceKeyTarget::Path(path),
    };

    shared_with_key(key, move |path| create(path))
}

pub fn shared_by_key<T>(
    kind: &'static str,
    key: impl Into<String>,
    create: impl Fn() -> Observable<T> + Send + Sync + 'static,
) -> Observable<T>
where
    T: Clone + Send + 'static,
{
    let key = SourceKey {
        kind,
        type_id: TypeId::of::<T>(),
        target: SourceKeyTarget::Descriptor(key.into()),
    };

    shared_with_key(key, move |_| create())
}

fn shared_with_key<T>(
    key: SourceKey,
    create: impl Fn(PathBuf) -> Observable<T> + Send + Sync + 'static,
) -> Observable<T>
where
    T: Clone + Send + 'static,
{
    let path = key.target.path().unwrap_or_default();

    let mut cache = source_cache().lock().expect("source cache lock poisoned");
    if let Some(hub) = cache
        .get(&key)
        .and_then(|value| value.downcast_ref::<Weak<ShareReplayHub<T>>>())
        .and_then(Weak::upgrade)
    {
        trace_source_lifecycle("cache hit", &source_key_label(&key));
        return share_replay_latest(hub);
    }

    trace_source_lifecycle("cache miss", &source_key_label(&key));
    let label = source_key_short_label(&key);
    let hub = Arc::new(ShareReplayHub::new(label, move || create(path.clone())));
    cache.insert(key, Box::new(Arc::downgrade(&hub)));
    share_replay_latest(hub)
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SourceKey {
    kind: &'static str,
    type_id: TypeId,
    target: SourceKeyTarget,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum SourceKeyTarget {
    Path(PathBuf),
    Descriptor(String),
}

impl SourceKeyTarget {
    fn path(&self) -> Option<PathBuf> {
        match self {
            Self::Path(path) => Some(path.clone()),
            Self::Descriptor(_) => None,
        }
    }
}

fn source_cache() -> &'static Mutex<HashMap<SourceKey, Box<dyn Any + Send>>> {
    static SOURCE_CACHE: OnceLock<Mutex<HashMap<SourceKey, Box<dyn Any + Send>>>> = OnceLock::new();
    SOURCE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn source_key_label(key: &SourceKey) -> String {
    format!("{} type={:?}", source_key_short_label(key), key.type_id)
}

fn source_key_short_label(key: &SourceKey) -> String {
    match &key.target {
        SourceKeyTarget::Path(path) => format!("{}:{}", key.kind, path.display()),
        SourceKeyTarget::Descriptor(descriptor) => format!("{}:{descriptor}", key.kind),
    }
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
            _hub: self.hub.clone(),
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
    _hub: Arc<ShareReplayHub<T>>,
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
    Parent {
        watch: locusfs_watch::Watch,
        ancestor: PathBuf,
    },
}

impl OpenedWatch {
    pub fn into_parts(self) -> (locusfs_watch::Watch, Option<PathBuf>) {
        match self {
            Self::Target(watch) => (watch, None),
            Self::Parent { watch, ancestor } => (watch, Some(ancestor)),
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
                .map(|watch| OpenedWatch::Parent {
                    watch,
                    ancestor: ancestor.clone(),
                })
                .map_err(|error| watch_error("open ancestor watch", &ancestor, error))
        }
        Err(error) => Err(watch_error("open watch", path, error)),
    }
}

pub fn ancestor_event_may_affect_path(ancestor: &Path, path: &Path, event: &WatchEvent) -> bool {
    let Some(target) = path_components(path.strip_prefix(ancestor).unwrap_or(path)) else {
        return true;
    };
    let Some(event_paths) = event_paths_relative_to_ancestor(ancestor, event) else {
        return true;
    };

    event_paths
        .iter()
        .any(|event_path| paths_intersect(event_path, &target))
}

fn event_paths_relative_to_ancestor(
    ancestor: &Path,
    event: &WatchEvent,
) -> Option<Vec<Vec<String>>> {
    match event {
        WatchEvent::State(_) => Some(Vec::new()),
        WatchEvent::Change(WatchChange::Change) => None,
        WatchEvent::Change(WatchChange::Node { node, .. }) => {
            Some(vec![absolute_event_path_relative_to_ancestor(
                ancestor,
                node_path_components(node)?,
            )])
        }
        WatchEvent::Change(WatchChange::Property { node, key, .. }) => {
            Some(vec![child_event_path_relative_to_ancestor(
                ancestor, node, key,
            )?])
        }
        WatchEvent::Change(WatchChange::Relation { node, relation, .. }) => {
            Some(vec![child_event_path_relative_to_ancestor(
                ancestor, node, relation,
            )?])
        }
    }
}

fn child_event_path_relative_to_ancestor(
    ancestor: &Path,
    node: &Option<String>,
    child: &str,
) -> Option<Vec<String>> {
    let mut path = match node {
        Some(node) => {
            absolute_event_path_relative_to_ancestor(ancestor, node_path_components(node)?)
        }
        None => Vec::new(),
    };
    path.push(child.to_owned());
    Some(path)
}

fn absolute_event_path_relative_to_ancestor(
    ancestor: &Path,
    mut event_path: Vec<String>,
) -> Vec<String> {
    let Some(ancestor) = path_components(ancestor) else {
        return event_path;
    };
    let strip_len = (0..=event_path.len())
        .rev()
        .find(|len| ancestor.ends_with(&event_path[..*len]))
        .unwrap_or(0);
    event_path.drain(..strip_len);
    event_path
}

fn node_path_components(node: &str) -> Option<Vec<String>> {
    let (kind, local) = node.split_once(':')?;
    Some(vec![kind.to_owned(), local.to_owned()])
}

fn path_components(path: &Path) -> Option<Vec<String>> {
    path.components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .map(|component| component.to_owned())
        })
        .collect()
}

fn paths_intersect(event_path: &[String], target: &[String]) -> bool {
    event_path.is_empty() || event_path.starts_with(target) || target.starts_with(event_path)
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
        convert::Infallible,
        path::Path,
        pin::Pin,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context as TaskContext, Poll},
    };

    use futures_util::Stream;
    use rxrust::prelude::{
        IntoBoxedSubscription as _, Observable as _, ObservableFactory as _, Observer, Shared,
        Subscription,
    };

    use crate::source::{WatchAction, WatchChange, WatchEvent, WatchState};

    use super::{
        ShareReplayHub, ShareReplayState, ShareReplaySubscription, ancestor_event_may_affect_path,
        from_stream_result, shared_by_key,
    };

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
            _hub: unused_i32_hub(),
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
            _hub: unused_i32_hub(),
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

    #[test]
    fn shared_by_key_reuses_active_semantic_source() {
        let create_count = Arc::new(AtomicUsize::new(0));
        let mut subject = Shared::subject::<u32, String>();
        let first_values = Arc::new(Mutex::new(Vec::new()));
        let second_values = Arc::new(Mutex::new(Vec::new()));

        let first = shared_by_key("test-derived", "same", {
            let create_count = create_count.clone();
            let subject = subject.clone();
            move || {
                create_count.fetch_add(1, Ordering::SeqCst);
                subject.clone().box_it()
            }
        });
        let second = shared_by_key("test-derived", "same", {
            let create_count = create_count.clone();
            let subject = subject.clone();
            move || {
                create_count.fetch_add(1, Ordering::SeqCst);
                subject.clone().box_it()
            }
        });

        let _first_subscription = first.subscribe_with(CollectU32(first_values.clone()));
        let _second_subscription = second.subscribe_with(CollectU32(second_values.clone()));

        assert_eq!(create_count.load(Ordering::SeqCst), 1);

        subject.next(7);

        assert_eq!(first_values.lock().unwrap().as_slice(), &[7]);
        assert_eq!(second_values.lock().unwrap().as_slice(), &[7]);
    }

    fn unused_i32_hub() -> Arc<ShareReplayHub<i32>> {
        Arc::new(ShareReplayHub::new("test:/source".to_owned(), || {
            Shared::<()>::of(0)
                .map_err(|error: Infallible| match error {})
                .box_it()
        }))
    }

    struct CollectU32(Arc<Mutex<Vec<u32>>>);

    impl Observer<u32, String> for CollectU32 {
        fn next(&mut self, value: u32) {
            self.0.lock().unwrap().push(value);
        }

        fn error(self, error: String) {
            panic!("unexpected observable error: {error}");
        }

        fn complete(self) {}

        fn is_closed(&self) -> bool {
            false
        }
    }

    #[test]
    fn ancestor_filter_ignores_unrelated_root_node_events() {
        let event = WatchEvent::Change(WatchChange::Node {
            action: WatchAction::Added,
            node: "window:1".to_owned(),
        });

        assert!(!ancestor_event_may_affect_path(
            Path::new("/run/locusfs"),
            Path::new("/run/locusfs/statusnotifier/item"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_ignores_root_node_events_for_same_kind_different_local() {
        let event = WatchEvent::Change(WatchChange::Node {
            action: WatchAction::Added,
            node: "statusnotifier:other".to_owned(),
        });

        assert!(!ancestor_event_may_affect_path(
            Path::new("/run/locusfs"),
            Path::new("/run/locusfs/statusnotifier/item"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_accepts_root_node_events_for_target() {
        let event = WatchEvent::Change(WatchChange::Node {
            action: WatchAction::Added,
            node: "statusnotifier:item".to_owned(),
        });

        assert!(ancestor_event_may_affect_path(
            Path::new("/run/locusfs"),
            Path::new("/run/locusfs/statusnotifier/item"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_ignores_unrelated_kind_node_events() {
        let event = WatchEvent::Change(WatchChange::Node {
            action: WatchAction::Added,
            node: "statusnotifier:other".to_owned(),
        });

        assert!(!ancestor_event_may_affect_path(
            Path::new("/run/locusfs/statusnotifier"),
            Path::new("/run/locusfs/statusnotifier/item"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_accepts_kind_node_events_for_target() {
        let event = WatchEvent::Change(WatchChange::Node {
            action: WatchAction::Added,
            node: "statusnotifier:item".to_owned(),
        });

        assert!(ancestor_event_may_affect_path(
            Path::new("/run/locusfs/statusnotifier"),
            Path::new("/run/locusfs/statusnotifier/item"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_matches_subject_relative_property_events() {
        let event = WatchEvent::Change(WatchChange::Property {
            action: WatchAction::Changed,
            node: None,
            key: "title".to_owned(),
        });

        assert!(ancestor_event_may_affect_path(
            Path::new("/run/locusfs/statusnotifier/item"),
            Path::new("/run/locusfs/statusnotifier/item/title"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_matches_absolute_property_events_under_kind() {
        let event = WatchEvent::Change(WatchChange::Property {
            action: WatchAction::Added,
            node: Some("statusnotifier:item".to_owned()),
            key: "title".to_owned(),
        });

        assert!(ancestor_event_may_affect_path(
            Path::new("/run/locusfs/statusnotifier"),
            Path::new("/run/locusfs/statusnotifier/item/title"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_ignores_unrelated_property_events_under_kind() {
        let event = WatchEvent::Change(WatchChange::Property {
            action: WatchAction::Added,
            node: Some("statusnotifier:other".to_owned()),
            key: "title".to_owned(),
        });

        assert!(!ancestor_event_may_affect_path(
            Path::new("/run/locusfs/statusnotifier"),
            Path::new("/run/locusfs/statusnotifier/item/title"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_accepts_generic_change_events() {
        let event = WatchEvent::Change(WatchChange::Change);

        assert!(ancestor_event_may_affect_path(
            Path::new("/run/locusfs"),
            Path::new("/run/locusfs/statusnotifier/item"),
            &event,
        ));
    }

    #[test]
    fn ancestor_filter_ignores_ancestor_state_events() {
        let event = WatchEvent::State(WatchState::Unset);

        assert!(!ancestor_event_may_affect_path(
            Path::new("/run/locusfs"),
            Path::new("/run/locusfs/statusnotifier/item"),
            &event,
        ));
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
