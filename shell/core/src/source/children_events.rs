use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::{Observable as _, ObservableFactory as _, Shared};

use crate::{locus_path::LocusPath, source::Observable};

use super::{
    ChildrenEvent, WatchAction, WatchChange, WatchEvent, WatchState,
    support::{is_missing, watch_error},
};

pub(super) fn children_events(path: impl Into<PathBuf>) -> Observable<ChildrenEvent> {
    Shared::<()>::from_stream_result(children_event_stream(path.into())).box_it()
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
    phase: ChildrenEventStreamPhase,
}

fn children_event_stream(
    path: PathBuf,
) -> impl futures_util::Stream<Item = Result<ChildrenEvent, String>> {
    stream::unfold(
        ChildrenEventStreamState {
            path,
            watch: None,
            phase: ChildrenEventStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    ChildrenEventStreamPhase::Open => {
                        match locusfs_watch::Watch::open(&state.path).await {
                            Ok(watch) => {
                                state.watch = Some(watch);
                                state.phase = ChildrenEventStreamPhase::InitialRead;
                            }
                            Err(error) => {
                                state.phase = ChildrenEventStreamPhase::Done;
                                let error =
                                    watch_error("open children event watch", &state.path, error);
                                return Some((Err(error), state));
                            }
                        }
                    }
                    ChildrenEventStreamPhase::InitialRead => {
                        state.phase = ChildrenEventStreamPhase::WatchEvents;
                        let result = read_children_snapshot(&state.path)
                            .await
                            .map(ChildrenEvent::Snapshot);
                        return Some((result, state));
                    }
                    ChildrenEventStreamPhase::WatchEvents => {
                        let result = match state
                            .watch
                            .as_mut()
                            .expect("watch initialized")
                            .next_event()
                            .await
                        {
                            Ok(event) => children_watch_event(&state.path, event).await,
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

async fn children_watch_event(path: &Path, event: WatchEvent) -> Result<ChildrenEvent, String> {
    match event {
        WatchEvent::State(WatchState::Unset) => Ok(ChildrenEvent::Snapshot(Vec::new())),
        WatchEvent::State(WatchState::Set(_)) | WatchEvent::Change(WatchChange::Change) => {
            read_children_snapshot(path)
                .await
                .map(ChildrenEvent::Snapshot)
        }
        WatchEvent::Change(WatchChange::Node { action, node }) => Ok(children_event_from_action(
            action,
            node_child_path(path, &node),
        )),
        WatchEvent::Change(WatchChange::Property { action, key, .. }) => Ok(
            children_event_from_action(action, LocusPath::new(path).child(key)),
        ),
        WatchEvent::Change(WatchChange::Relation {
            action, relation, ..
        }) => Ok(children_event_from_action(
            action,
            LocusPath::new(path).child(relation),
        )),
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
    let Some((kind, local)) = node.split_once(':') else {
        return LocusPath::new(parent).child(node);
    };
    if parent.file_name().and_then(|name| name.to_str()) == Some(kind) {
        LocusPath::new(parent).child(local)
    } else {
        LocusPath::new(parent).child(node)
    }
}

async fn read_children_snapshot(path: &Path) -> Result<Vec<LocusPath>, String> {
    match locusfs_watch::read_dir_names(path).await {
        Ok(mut entries) => {
            entries.sort();
            let parent = LocusPath::new(path);
            Ok(entries
                .into_iter()
                .map(|entry| parent.child(entry))
                .collect())
        }
        Err(error) if is_missing(&error) => Ok(Vec::new()),
        Err(error) => Err(watch_error("read children snapshot", path, error)),
    }
}
