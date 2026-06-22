use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::Observable as _;

use crate::source::Observable;

use super::{
    NodeState, WatchAction, WatchChange, WatchEvent, WatchState,
    support::{WatchEvents, from_stream_result, log_errors, open_target_or_parent, watch_error},
};

pub(super) fn node(path: impl Into<PathBuf>) -> Observable<NodeState> {
    let path = path.into();
    let observable = from_stream_result(node_stream(path.clone()))
        .distinct_until_changed()
        .box_it();
    log_errors("node", path, observable)
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
    events: WatchEvents,
    watching_target: bool,
    phase: NodeStreamPhase,
}

fn node_stream(path: PathBuf) -> impl futures_util::Stream<Item = Result<NodeState, String>> {
    stream::unfold(
        NodeStreamState {
            path,
            watch: None,
            events: WatchEvents::new(),
            watching_target: false,
            phase: NodeStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    NodeStreamPhase::Open => match open_target_or_parent(&state.path).await {
                        Ok(opened) => {
                            let (watch, watching_target) = opened.into_parts();
                            state.watch = Some(watch);
                            state.watching_target = watching_target;
                            state.phase = NodeStreamPhase::InitialRead;
                        }
                        Err(error) => {
                            state.phase = NodeStreamPhase::Done;
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
                                    Ok(read_node_state(&state.path).await)
                                } else {
                                    let missing = matches!(
                                        event,
                                        WatchEvent::State(WatchState::Unset)
                                            | WatchEvent::Change(WatchChange::Node {
                                                action: WatchAction::Removed,
                                                ..
                                            })
                                    );
                                    let result = Ok(node_event_state(&state.path, event).await);
                                    if missing {
                                        state.watch = None;
                                        state.watching_target = false;
                                        state.phase = NodeStreamPhase::Open;
                                    }
                                    result
                                }
                            }
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
