use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::{Observable as _, ObservableFactory as _, Shared};

use crate::{locus_path::LocusPath, source::Observable};

use super::{
    WatchEvent, WatchState,
    support::{WatchEvents, is_missing, log_errors, open_target_or_parent, watch_error},
};

pub(super) fn children(path: impl Into<PathBuf>) -> Observable<Vec<LocusPath>> {
    let path = path.into();
    let observable = Shared::<()>::from_stream_result(children_stream(path.clone()))
        .distinct_until_changed()
        .box_it();
    log_errors("children", path, observable)
}

enum ChildrenStreamPhase {
    Open,
    InitialRead,
    WatchEvents,
    Done,
}

struct ChildrenStreamState {
    path: PathBuf,
    watch: Option<locusfs_watch::Watch>,
    events: WatchEvents,
    watching_target: bool,
    phase: ChildrenStreamPhase,
}

fn children_stream(
    path: PathBuf,
) -> impl futures_util::Stream<Item = Result<Vec<LocusPath>, String>> {
    stream::unfold(
        ChildrenStreamState {
            path,
            watch: None,
            events: WatchEvents::new(),
            watching_target: false,
            phase: ChildrenStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    ChildrenStreamPhase::Open => match open_target_or_parent(&state.path).await {
                        Ok(opened) => {
                            let (watch, watching_target) = opened.into_parts();
                            state.watch = Some(watch);
                            state.watching_target = watching_target;
                            state.phase = ChildrenStreamPhase::InitialRead;
                        }
                        Err(error) => {
                            state.phase = ChildrenStreamPhase::Done;
                            return Some((Err(error), state));
                        }
                    },
                    ChildrenStreamPhase::InitialRead => {
                        state.phase = ChildrenStreamPhase::WatchEvents;
                        let result = read_children(&state.path).await;
                        return Some((result, state));
                    }
                    ChildrenStreamPhase::WatchEvents => {
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
                                    read_children(&state.path).await
                                } else {
                                    let unset =
                                        matches!(event, WatchEvent::State(WatchState::Unset));
                                    let result = children_event_value(&state.path, event).await;
                                    if unset {
                                        state.watch = None;
                                        state.watching_target = false;
                                        state.phase = ChildrenStreamPhase::Open;
                                    }
                                    result
                                }
                            }
                            Err(error) => {
                                Err(watch_error("read children watch event", &state.path, error))
                            }
                        };

                        if result.is_err() {
                            state.phase = ChildrenStreamPhase::Done;
                        }

                        return Some((result, state));
                    }
                    ChildrenStreamPhase::Done => return None,
                }
            }
        },
    )
}

async fn children_event_value(path: &Path, event: WatchEvent) -> Result<Vec<LocusPath>, String> {
    match event {
        WatchEvent::State(WatchState::Unset) => Ok(Vec::new()),
        WatchEvent::State(WatchState::Set(_)) | WatchEvent::Change(_) => read_children(path).await,
    }
}

async fn read_children(path: &Path) -> Result<Vec<LocusPath>, String> {
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
        Err(error) => Err(watch_error("read children", path, error)),
    }
}
