use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::{Observable as _, ObservableFactory as _, Shared};

use crate::{locus_path::LocusPath, source::Observable};

use super::{
    WatchEvent, WatchState,
    support::{is_missing, watch_error},
};

pub(super) fn children(path: impl Into<PathBuf>) -> Observable<Vec<LocusPath>> {
    Shared::<()>::from_stream_result(children_stream(path.into()))
        .distinct_until_changed()
        .box_it()
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
    phase: ChildrenStreamPhase,
}

fn children_stream(
    path: PathBuf,
) -> impl futures_util::Stream<Item = Result<Vec<LocusPath>, String>> {
    stream::unfold(
        ChildrenStreamState {
            path,
            watch: None,
            phase: ChildrenStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    ChildrenStreamPhase::Open => {
                        match locusfs_watch::Watch::open(&state.path).await {
                            Ok(watch) => {
                                state.watch = Some(watch);
                                state.phase = ChildrenStreamPhase::InitialRead;
                            }
                            Err(error) => {
                                state.phase = ChildrenStreamPhase::Done;
                                let error = watch_error("open children watch", &state.path, error);
                                return Some((Err(error), state));
                            }
                        }
                    }
                    ChildrenStreamPhase::InitialRead => {
                        state.phase = ChildrenStreamPhase::WatchEvents;
                        let result = read_children(&state.path).await;
                        return Some((result, state));
                    }
                    ChildrenStreamPhase::WatchEvents => {
                        let result = match state
                            .watch
                            .as_mut()
                            .expect("watch initialized")
                            .next_event()
                            .await
                        {
                            Ok(event) => children_event_value(&state.path, event).await,
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
