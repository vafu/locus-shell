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

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";

/// Returns the configured locusfs mount root.
///
/// This is a path-construction helper for observable source functions.
pub fn root() -> LocusPath {
    LocusPath::from_env_or(ROOT_ENV, DEFAULT_ROOT)
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
pub fn combine_latest_vec<T>(observables: Vec<Observable<T>>) -> Observable<Vec<T>>
where
    T: Clone + Send + 'static,
{
    let mut observables = observables.into_iter();
    let Some(first) = observables.next() else {
        return once(Vec::new());
    };

    observables.fold(
        first.map(|value| vec![value]).box_it(),
        |combined, observable| {
            combined
                .combine_latest(observable, |mut values, value| {
                    values.push(value);
                    values
                })
                .box_it()
        },
    )
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
