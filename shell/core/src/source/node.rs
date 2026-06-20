use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::{Observable as _, ObservableFactory as _, Shared};

use crate::source::Observable;

use super::{NodeState, WatchAction, WatchChange, WatchEvent, WatchState, support::watch_error};

pub(super) fn node(path: impl Into<PathBuf>) -> Observable<NodeState> {
    Shared::<()>::from_stream_result(node_stream(path.into()))
        .distinct_until_changed()
        .box_it()
}

enum NodeStreamPhase {
    Open,
    InitialRead,
    WatchEvents,
    Done,
}

struct NodeStreamState {
    path: PathBuf,
    watch: Option<locusfs_watch::Watch>,
    phase: NodeStreamPhase,
}

fn node_stream(path: PathBuf) -> impl futures_util::Stream<Item = Result<NodeState, String>> {
    stream::unfold(
        NodeStreamState {
            path,
            watch: None,
            phase: NodeStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    NodeStreamPhase::Open => match locusfs_watch::Watch::open(&state.path).await {
                        Ok(watch) => {
                            state.watch = Some(watch);
                            state.phase = NodeStreamPhase::InitialRead;
                        }
                        Err(error) => {
                            state.phase = NodeStreamPhase::Done;
                            let error = watch_error("open node watch", &state.path, error);
                            return Some((Err(error), state));
                        }
                    },
                    NodeStreamPhase::InitialRead => {
                        state.phase = NodeStreamPhase::WatchEvents;
                        let result = read_node_state(&state.path).await;
                        return Some((Ok(result), state));
                    }
                    NodeStreamPhase::WatchEvents => {
                        let result = match state
                            .watch
                            .as_mut()
                            .expect("watch initialized")
                            .next_event()
                            .await
                        {
                            Ok(event) => Ok(node_event_state(&state.path, event).await),
                            Err(error) => {
                                Err(watch_error("read node watch event", &state.path, error))
                            }
                        };

                        if result.is_err() {
                            state.phase = NodeStreamPhase::Done;
                        }

                        return Some((result, state));
                    }
                    NodeStreamPhase::Done => return None,
                }
            }
        },
    )
}

async fn node_event_state(path: &Path, event: WatchEvent) -> NodeState {
    match event {
        WatchEvent::State(WatchState::Unset) => NodeState::Missing,
        WatchEvent::State(WatchState::Set(_)) => NodeState::Present,
        WatchEvent::Change(WatchChange::Node {
            action: WatchAction::Removed,
            ..
        }) => NodeState::Missing,
        WatchEvent::Change(WatchChange::Node { .. }) => NodeState::Present,
        WatchEvent::Change(WatchChange::Change)
        | WatchEvent::Change(WatchChange::Property { .. })
        | WatchEvent::Change(WatchChange::Relation { .. }) => read_node_state(path).await,
    }
}

async fn read_node_state(path: &Path) -> NodeState {
    if locusfs_watch::exists(path).await {
        NodeState::Present
    } else {
        NodeState::Missing
    }
}
