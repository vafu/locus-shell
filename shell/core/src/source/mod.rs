use std::{
    convert::Infallible,
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::locus_path::LocusPath;
use rxrust::observer::{BoxedObserverSend, Observer as _};
use rxrust::prelude::{
    Observable as RxObservable, ObservableFactory as _, Shared, SharedBoxedObservable,
};

mod async_loop;
mod children;
mod children_events;
mod conversion;
mod legacy;
mod node;
mod property;
mod relation;
mod support;
mod watch;

pub use locusfs_watch::{WatchAction, WatchChange, WatchEvent, WatchState, WatchValue};
pub use rxrust;
pub use rxrust::prelude as rx;

pub type Observable<T, E = String> = SharedBoxedObservable<'static, T, E>;

pub fn once<T>(value: T) -> Observable<T, Infallible>
where
    T: Send + 'static,
{
    Shared::<()>::of(value).box_it()
}

/// Legacy async bridge for handwritten emitter loops.
///
/// New source primitives should use RxRust-native factories such as
/// `Shared::from_stream_result` or `Shared::from_future_result` near the
/// backend implementation.
#[deprecated(note = "legacy async-loop bridge; prefer RxRust-native stream/future factories")]
pub fn from_async_loop<T, Run, RunFuture>(run: Run) -> Observable<T>
where
    T: Send + 'static,
    Run: FnOnce(AsyncEmitter<T>) -> RunFuture + Send + 'static,
    RunFuture: Future<Output = ()> + Send + 'static,
{
    async_loop::from_async_loop(run)
}

/// Legacy emitter used by [`from_async_loop`].
///
/// New source code should model event production as a stream/future and convert
/// it with RxRust factories near the backend implementation.
pub struct AsyncEmitter<T, E = String> {
    observer: Arc<Mutex<Option<BoxedObserverSend<'static, T, E>>>>,
}

impl<T, E> Clone for AsyncEmitter<T, E> {
    fn clone(&self) -> Self {
        Self {
            observer: self.observer.clone(),
        }
    }
}

impl<T, E> AsyncEmitter<T, E> {
    fn new(observer: BoxedObserverSend<'static, T, E>) -> Self {
        Self {
            observer: Arc::new(Mutex::new(Some(observer))),
        }
    }

    pub fn next(&self, value: T) {
        let Ok(mut observer) = self.observer.lock() else {
            return;
        };
        if let Some(observer) = observer.as_mut() {
            observer.next(value);
        }
    }

    pub fn error(&self, error: E) {
        let Ok(mut observer) = self.observer.lock() else {
            return;
        };
        if let Some(observer) = observer.take() {
            observer.error(error);
        }
    }

    pub fn complete(&self) {
        let Ok(mut observer) = self.observer.lock() else {
            return;
        };
        if let Some(observer) = observer.take() {
            observer.complete();
        }
    }
}

/// State of a locusfs node path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeState {
    Present,
    Missing,
}

/// State of a locusfs relation/symlink path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationState {
    Set(LocusPath),
    Unset,
}

/// Child update event for a watched locusfs path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildrenEvent {
    Snapshot(Vec<LocusPath>),
    Added(LocusPath),
    Changed(LocusPath),
    Removed(LocusPath),
}

/// Conversion from locusfs property text into a typed Rust value.
pub trait FromLocusValue: Clone + PartialEq + Send + 'static {
    fn from_locus_value(value: &str) -> Result<Self, String>;
}

/// Legacy watch event used by the temporary multi-watch helper APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyWatchEvent {
    path: Option<PathBuf>,
    message: String,
}

impl LegacyWatchEvent {
    fn initial() -> Self {
        Self {
            path: None,
            message: "initial".to_owned(),
        }
    }

    fn new(path: PathBuf, message: String) -> Self {
        Self {
            path: Some(path),
            message,
        }
    }

    pub fn is_initial(&self) -> bool {
        self.path.is_none()
    }

    pub fn is_unset(&self) -> bool {
        self.event_name() == Some("unset")
    }

    pub fn is_set(&self) -> bool {
        self.event_name() == Some("set")
    }

    pub fn resolved_path(&self) -> Option<&Path> {
        let path = self
            .message
            .strip_prefix("set ")
            .filter(|path| !path.is_empty())?;
        Some(Path::new(path))
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn event_name(&self) -> Option<&str> {
        self.message.split_whitespace().next()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WatchTarget {
    Value,
    Directory,
}

/// Legacy watch descriptor used by the temporary multi-watch helper APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchSpec {
    path: PathBuf,
    target: WatchTarget,
    required: bool,
}

impl WatchSpec {
    pub fn value(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Value,
            required: true,
        }
    }

    pub fn directory(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Directory,
            required: true,
        }
    }

    pub fn optional_value(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Value,
            required: false,
        }
    }

    pub fn optional_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Directory,
            required: false,
        }
    }
}

/// Emits typed events for one locusfs watch path.
///
/// TODO: add descriptor-keyed share/replay so multiple subscribers do not open
/// duplicate `/watch` file descriptors.
pub fn watch(path: impl Into<PathBuf>) -> Observable<WatchEvent> {
    watch::watch(path)
}

/// Emits the current typed property value and future updates for one property.
///
/// `None` means the property is currently absent/unset.
/// Consecutive duplicate values are suppressed.
///
/// TODO: add descriptor-keyed share/replay by `(path, T)`.
pub fn property<T>(path: impl Into<PathBuf>) -> Observable<Option<T>>
where
    T: FromLocusValue,
{
    property::property(path)
}

/// Emits the current relation target and future updates for one relation path.
///
/// `None` means the relation is currently unset.
/// Consecutive duplicate targets are suppressed.
///
/// TODO: add descriptor-keyed share/replay by path.
pub fn relation(path: impl Into<PathBuf>) -> Observable<Option<LocusPath>> {
    relation::relation(path)
}

/// Emits the current node state and future updates for one node path.
///
/// TODO: add descriptor-keyed share/replay by path.
pub fn node(path: impl Into<PathBuf>) -> Observable<NodeState> {
    node::node(path)
}

/// Emits the current child path snapshot and future snapshots for one locusfs path.
///
/// This is the convenient list-valued API for UI sources. Each item is a full
/// child path, so consumers can keep composing with `LocusPath::prop` and
/// `LocusPath::rel`.
/// Consecutive duplicate snapshots are suppressed.
pub fn children(path: impl Into<PathBuf>) -> Observable<Vec<LocusPath>> {
    children::children(path)
}

/// Emits child-level change events for one locusfs path.
///
/// Use this when the consumer needs incremental event semantics rather than a
/// full snapshot.
pub fn children_events(path: impl Into<PathBuf>) -> Observable<ChildrenEvent> {
    children_events::children_events(path)
}

/// Legacy multi-watch event stream.
///
/// Prefer the path-specific Rx helpers above for new source implementations.
pub fn change_events_async<Open, OpenFuture>(open_watches: Open) -> Observable<LegacyWatchEvent>
where
    Open: FnMut() -> OpenFuture + Send + 'static,
    OpenFuture: Future<Output = Result<Vec<WatchSpec>, String>> + Send,
{
    legacy::change_events_async_impl(open_watches)
}

/// Legacy read-on-change helper for one watch descriptor.
///
/// Prefer the path-specific Rx helpers above for new source implementations.
pub fn read_on_change_async<Value, Read, ReadFuture>(
    watch: WatchSpec,
    read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, String>> + Send,
{
    legacy::read_on_change_async_impl(watch, read)
}

/// Legacy read-on-any-change helper for a dynamic watch set.
///
/// Prefer the path-specific Rx helpers above for new source implementations.
pub fn read_on_any_change_async<Value, Open, OpenFuture, Read, ReadFuture>(
    open_watches: Open,
    read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Open: FnMut() -> OpenFuture + Send + 'static,
    OpenFuture: Future<Output = Result<Vec<WatchSpec>, String>> + Send,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, String>> + Send,
{
    legacy::read_on_any_change_async_impl(open_watches, read)
}
