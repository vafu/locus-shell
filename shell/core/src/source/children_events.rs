use std::path::{Path, PathBuf};

use futures_util::stream;

use crate::{locus_path::LocusPath, source::Observable};

use super::{
    ChildrenEvent, WatchAction, WatchChange, WatchEvent, WatchState,
    support::{
        WatchEvents, from_stream_result, is_missing, log_errors, open_target_or_parent,
        shared_source, watch_error,
    },
};

pub(super) fn children_events(path: impl Into<PathBuf>) -> Observable<ChildrenEvent> {
    let path = path.into();
    shared_source("children_events", path, |path| {
        let observable = from_stream_result(children_event_stream(path.clone()));
        log_errors("children_events", path, observable)
    })
}

enum ChildrenEventStreamPhase {
    Open,
    InitialRead,
    WatchEvents,
    Done,
}

struct ChildrenEventStreamState {
    path: PathBuf,
    watch: Option<locusfs_watch::Watch>,
    events: WatchEvents,
    watching_target: bool,
    phase: ChildrenEventStreamPhase,
}

fn children_event_stream(
    path: PathBuf,
) -> impl futures_util::Stream<Item = Result<ChildrenEvent, String>> {
    stream::unfold(
        ChildrenEventStreamState {
            path,
            watch: None,
            events: WatchEvents::new(),
            watching_target: false,
            phase: ChildrenEventStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    ChildrenEventStreamPhase::Open => {
                        match open_target_or_parent(&state.path).await {
                            Ok(opened) => {
                                let (watch, watching_target) = opened.into_parts();
                                state.watch = Some(watch);
                                state.watching_target = watching_target;
                                state.phase = ChildrenEventStreamPhase::InitialRead;
                            }
                            Err(error) => {
                                state.phase = ChildrenEventStreamPhase::Done;
                                return Some((Err(error), state));
                            }
                        }
                    }
                    ChildrenEventStreamPhase::InitialRead => {
                        state.phase = ChildrenEventStreamPhase::WatchEvents;
                        let result = read_children_snapshot_state(&state.path).await;
                        if state.watching_target
                            && matches!(result, Ok(ChildrenSnapshotRead::Missing))
                        {
                            state.watch = None;
                            state.watching_target = false;
                            state.phase = ChildrenEventStreamPhase::Open;
                        }
                        let result = result.map(ChildrenSnapshotRead::into_event);
                        return Some((result, state));
                    }
                    ChildrenEventStreamPhase::WatchEvents => {
                        let result = match state
                            .events
                            .next(state.watch.as_mut().expect("watch initialized"))
                            .await
                        {
                            Ok(event) => {
                                if !state.watching_target {
                                    if let Ok(opened) = open_target_or_parent(&state.path).await {
                                        let (watch, watching_target) = opened.into_parts();
                                        state.watch = Some(watch);
                                        state.watching_target = watching_target;
                                    }
                                    read_children_snapshot(&state.path)
                                        .await
                                        .map(ChildrenEvent::Snapshot)
                                } else {
                                    let unset =
                                        matches!(event, WatchEvent::State(WatchState::Unset));
                                    let result = children_watch_event(&state.path, event).await;
                                    if children_event_watch_should_reopen(unset, &result) {
                                        state.watch = None;
                                        state.watching_target = false;
                                        state.phase = ChildrenEventStreamPhase::Open;
                                    }
                                    result.map(ChildrenSnapshotRead::into_event)
                                }
                            }
                            Err(error) => Err(watch_error(
                                "read children event watch event",
                                &state.path,
                                error,
                            )),
                        };

                        if result.is_err() {
                            state.phase = ChildrenEventStreamPhase::Done;
                        }

                        return Some((result, state));
                    }
                    ChildrenEventStreamPhase::Done => return None,
                }
            }
        },
    )
}

enum ChildrenSnapshotRead {
    Event(ChildrenEvent),
    Snapshot(Vec<LocusPath>),
    Missing,
}

impl ChildrenSnapshotRead {
    fn into_event(self) -> ChildrenEvent {
        match self {
            Self::Event(event) => event,
            Self::Snapshot(children) => ChildrenEvent::Snapshot(children),
            Self::Missing => ChildrenEvent::Snapshot(Vec::new()),
        }
    }
}

async fn children_watch_event(
    path: &Path,
    event: WatchEvent,
) -> Result<ChildrenSnapshotRead, String> {
    match event {
        WatchEvent::State(WatchState::Unset) => Ok(ChildrenSnapshotRead::Missing),
        WatchEvent::State(WatchState::Set(_)) | WatchEvent::Change(WatchChange::Change) => {
            read_children_snapshot_state(path).await
        }
        WatchEvent::Change(WatchChange::Node { action, node }) => Ok(ChildrenSnapshotRead::Event(
            children_event_from_action(action, node_child_path(path, &node)),
        )),
        WatchEvent::Change(WatchChange::Property { action, key, .. }) => {
            Ok(ChildrenSnapshotRead::Event(children_event_from_action(
                action,
                LocusPath::new(path).child(key),
            )))
        }
        WatchEvent::Change(WatchChange::Relation {
            action, relation, ..
        }) => Ok(ChildrenSnapshotRead::Event(children_event_from_action(
            action,
            LocusPath::new(path).child(relation),
        ))),
    }
}

fn children_event_from_action(action: WatchAction, child: LocusPath) -> ChildrenEvent {
    match action {
        WatchAction::Added => ChildrenEvent::Added(child),
        WatchAction::Changed => ChildrenEvent::Changed(child),
        WatchAction::Removed => ChildrenEvent::Removed(child),
    }
}

fn node_child_path(parent: &Path, node: &str) -> LocusPath {
    let Some((_, local)) = node.split_once(':') else {
        return LocusPath::new(parent).child(node);
    };
    LocusPath::new(parent).child(local)
}

async fn read_children_snapshot_state(path: &Path) -> Result<ChildrenSnapshotRead, String> {
    match locusfs_watch::read_dir_names(path).await {
        Ok(mut entries) => {
            entries.sort();
            let parent = LocusPath::new(path);
            Ok(ChildrenSnapshotRead::Snapshot(
                entries
                    .into_iter()
                    .map(|entry| parent.child(entry))
                    .collect(),
            ))
        }
        Err(error) if is_missing(&error) => Ok(ChildrenSnapshotRead::Missing),
        Err(error) => Err(watch_error("read children snapshot", path, error)),
    }
}

async fn read_children_snapshot(path: &Path) -> Result<Vec<LocusPath>, String> {
    read_children_snapshot_state(path)
        .await
        .map(|read| match read {
            ChildrenSnapshotRead::Snapshot(children) => children,
            ChildrenSnapshotRead::Missing | ChildrenSnapshotRead::Event(_) => Vec::new(),
        })
}

fn children_event_watch_should_reopen(
    missing_event: bool,
    result: &Result<ChildrenSnapshotRead, String>,
) -> bool {
    missing_event || matches!(result, Ok(ChildrenSnapshotRead::Missing))
}

#[cfg(test)]
mod tests {
    use super::{ChildrenSnapshotRead, children_event_watch_should_reopen};

    #[test]
    fn children_event_watch_reopens_on_explicit_missing_event() {
        assert!(children_event_watch_should_reopen(
            true,
            &Ok(ChildrenSnapshotRead::Snapshot(Vec::new())),
        ));
    }

    #[test]
    fn children_event_watch_reopens_when_event_read_finds_missing_directory() {
        assert!(children_event_watch_should_reopen(
            false,
            &Ok(ChildrenSnapshotRead::Missing),
        ));
    }

    #[test]
    fn children_event_watch_does_not_reopen_for_empty_present_directory() {
        assert!(!children_event_watch_should_reopen(
            false,
            &Ok(ChildrenSnapshotRead::Snapshot(Vec::new())),
        ));
    }
}
