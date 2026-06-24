use std::{
    convert::Infallible,
    path::{Path, PathBuf},
};

use crate::locus_path::LocusPath;
use rxrust::prelude::{
    Observable as RxObservable, ObservableFactory as _, Shared, SharedBoxedObservable,
};

mod children;
mod children_events;
mod conversion;
mod node;
mod property;
mod relation;
mod support;
mod watch;

pub use locusfs_watch::{WatchAction, WatchChange, WatchEvent, WatchState, WatchValue};
pub use rxrust;
pub use rxrust::prelude as rx;

pub type Observable<T, E = String> = SharedBoxedObservable<'static, T, E>;

const ROOT_ENV: &str = "LOCUS_ROOT";
const LEGACY_ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";

/// Returns the configured locusfs mount root.
///
/// This is a path-construction helper for observable source functions.
pub fn root() -> LocusPath {
    std::env::var_os(ROOT_ENV)
        .or_else(|| std::env::var_os(LEGACY_ROOT_ENV))
        .map(PathBuf::from)
        .or_else(runtime_locusfs_root)
        .map(LocusPath::new)
        .unwrap_or_else(|| LocusPath::new(DEFAULT_ROOT))
}

fn runtime_locusfs_root() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join("locusfs");
    root.exists().then_some(root)
}

impl LocusPath {
    /// Creates an observable for raw watch events on this path.
    pub fn as_watch(&self) -> Observable<WatchEvent> {
        watch(self)
    }

    /// Creates an observable for this path as a typed locusfs property.
    pub fn as_property<T>(&self) -> Observable<Option<T>>
    where
        T: FromLocusValue,
    {
        property(self)
    }

    /// Creates an observable for this path as a typed locusfs property,
    /// substituting `default` while the property is absent/unset.
    pub fn as_property_or<T>(&self, default: T) -> Observable<T>
    where
        T: FromLocusValue,
    {
        self.as_property()
            .map(move |value| value.unwrap_or(default.clone()))
            .box_it()
    }

    /// Creates an observable for a typed property under this path.
    pub fn observe_prop<T>(&self, property: impl AsRef<Path>) -> Observable<Option<T>>
    where
        T: FromLocusValue,
    {
        self.prop(property).as_property()
    }

    /// Creates an observable for a typed property under this path,
    /// substituting `default` while the property is absent/unset.
    pub fn observe_prop_or<T>(&self, property: impl AsRef<Path>, default: T) -> Observable<T>
    where
        T: FromLocusValue,
    {
        self.prop(property).as_property_or(default)
    }

    /// Creates an observable for this path as a locusfs relation/symlink.
    pub fn as_relation(&self) -> Observable<Option<LocusPath>> {
        relation(self)
    }

    /// Creates an observable for this path as a locusfs relation/symlink,
    /// substituting `default` while the relation is unset.
    pub fn as_relation_or(&self, default: LocusPath) -> Observable<LocusPath> {
        self.as_relation()
            .map(move |value| value.unwrap_or_else(|| default.clone()))
            .box_it()
    }

    /// Creates an observable for a relation under this path.
    pub fn observe_rel(&self, relation: impl AsRef<Path>) -> Observable<Option<LocusPath>> {
        self.rel(relation).as_relation()
    }

    /// Creates an observable for a relation under this path, substituting
    /// `default` while the relation is unset.
    pub fn observe_rel_or(
        &self,
        relation: impl AsRef<Path>,
        default: LocusPath,
    ) -> Observable<LocusPath> {
        self.rel(relation).as_relation_or(default)
    }

    /// Creates an observable for this path as a locusfs node.
    pub fn as_node(&self) -> Observable<NodeState> {
        node(self)
    }

    /// Creates an observable for this path's current children.
    pub fn as_children(&self) -> Observable<Vec<LocusPath>> {
        children(self)
    }

    /// Creates an observable for this path's child update events.
    pub fn as_children_events(&self) -> Observable<ChildrenEvent> {
        children_events(self)
    }
}

pub fn once<T>(value: T) -> Observable<T>
where
    T: Send + 'static,
{
    Shared::<()>::of(value)
        .map_err(|error: Infallible| match error {})
        .box_it()
}

/// Combines a dynamic list of observables into one latest-value vector.
///
/// RxRust currently exposes binary `combine_latest`; this keeps the fold in one
/// place while preserving normal Observable composition at call sites.
pub fn combine_latest<T>(observables: Vec<Observable<T>>) -> Observable<Vec<T>>
where
    T: Clone + Send + 'static,
{
    let len = observables.len();
    let mut observables = observables.into_iter().enumerate();
    let Some((first_index, first)) = observables.next() else {
        return once(Vec::new());
    };

    observables
        .fold(
            first
                .map(move |value| {
                    let mut values = vec![None; len];
                    values[first_index] = Some(value);
                    values
                })
                .box_it(),
            |combined, (index, observable)| {
                combined
                    .combine_latest(observable, move |mut values, value| {
                        values[index] = Some(value);
                        values
                    })
                    .box_it()
            },
        )
        .filter_map(|values| values.into_iter().collect::<Option<Vec<_>>>())
        .box_it()
}

/// Compatibility spelling for call sites that predate `combine_latest`.
pub fn combine_latest_vec<T>(observables: Vec<Observable<T>>) -> Observable<Vec<T>>
where
    T: Clone + Send + 'static,
{
    combine_latest(observables)
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

/// One source error caught by shell-core source primitives.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceError {
    pub id: u64,
    pub source: &'static str,
    pub path: PathBuf,
    pub message: String,
}

/// Process-local source error state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceErrors {
    pub total: u64,
    pub recent: Vec<SourceError>,
}

/// Conversion from locusfs property text into a typed Rust value.
pub trait FromLocusValue: Clone + PartialEq + Send + 'static {
    fn from_locus_value(value: &str) -> Result<Self, String>;
}

/// Emits typed events for one locusfs watch path.
pub fn watch(path: impl Into<PathBuf>) -> Observable<WatchEvent> {
    watch::watch(path)
}

/// Emits the current typed property value and future updates for one property.
///
/// `None` means the property is currently absent/unset.
/// Consecutive duplicate values are suppressed.
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
pub fn relation(path: impl Into<PathBuf>) -> Observable<Option<LocusPath>> {
    relation::relation(path)
}

/// Emits the current node state and future updates for one node path.
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

/// Emits the process-local total number of source errors caught by shell-core.
///
/// The first emission is the current total. Future emissions happen when a
/// source primitive logs a hard error.
pub fn error_count() -> Observable<u64> {
    support::error_count()
}

/// Emits process-local source error totals and recent error history.
pub fn errors() -> Observable<SourceErrors> {
    support::errors()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use rxrust::prelude::{Observable as _, ObservableFactory as _, Observer};

    use super::{Shared, combine_latest_vec};

    #[test]
    fn combine_latest_vec_replaces_values_by_source_index() {
        let mut first = Shared::subject::<i32, String>();
        let mut second = Shared::subject::<i32, String>();
        let emitted = Arc::new(Mutex::new(Vec::new()));

        let _subscription =
            combine_latest_vec(vec![first.clone().box_it(), second.clone().box_it()])
                .subscribe_with(CollectValues(emitted.clone()));

        first.next(1);
        assert!(emitted.lock().unwrap().is_empty());

        second.next(10);
        second.next(20);
        first.next(2);

        assert_eq!(
            emitted.lock().unwrap().as_slice(),
            &[vec![1, 10], vec![1, 20], vec![2, 20]]
        );
    }

    struct CollectValues(Arc<Mutex<Vec<Vec<i32>>>>);

    impl Observer<Vec<i32>, String> for CollectValues {
        fn next(&mut self, value: Vec<i32>) {
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
}
