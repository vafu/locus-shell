use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::{Observable as _, ObservableFactory as _, Shared};

use crate::{locus_path::LocusPath, source::Observable};

use super::{
    WatchEvent, WatchState, WatchValue,
    support::{is_missing, watch_error},
};

pub(super) fn relation(path: impl Into<PathBuf>) -> Observable<Option<LocusPath>> {
    Shared::<()>::from_stream_result(relation_stream(path.into()))
        .distinct_until_changed()
        .box_it()
}

enum RelationStreamPhase {
    Open,
    InitialRead,
    WatchEvents,
    Done,
}

struct RelationStreamState {
    path: PathBuf,
    watch: Option<locusfs_watch::Watch>,
    phase: RelationStreamPhase,
}

fn relation_stream(
    path: PathBuf,
) -> impl futures_util::Stream<Item = Result<Option<LocusPath>, String>> {
    stream::unfold(
        RelationStreamState {
            path,
            watch: None,
            phase: RelationStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    RelationStreamPhase::Open => {
                        match locusfs_watch::Watch::open(&state.path).await {
                            Ok(watch) => {
                                state.watch = Some(watch);
                                state.phase = RelationStreamPhase::InitialRead;
                            }
                            Err(error) => {
                                state.phase = RelationStreamPhase::Done;
                                let error = watch_error("open relation watch", &state.path, error);
                                return Some((Err(error), state));
                            }
                        }
                    }
                    RelationStreamPhase::InitialRead => {
                        state.phase = RelationStreamPhase::WatchEvents;
                        let result = read_relation(&state.path).await;
                        return Some((result, state));
                    }
                    RelationStreamPhase::WatchEvents => {
                        let result = match state
                            .watch
                            .as_mut()
                            .expect("watch initialized")
                            .next_event()
                            .await
                        {
                            Ok(event) => relation_event_value(&state.path, event).await,
                            Err(error) => {
                                Err(watch_error("read relation watch event", &state.path, error))
                            }
                        };

                        if result.is_err() {
                            state.phase = RelationStreamPhase::Done;
                        }

                        return Some((result, state));
                    }
                    RelationStreamPhase::Done => return None,
                }
            }
        },
    )
}

async fn relation_event_value(path: &Path, event: WatchEvent) -> Result<Option<LocusPath>, String> {
    match event {
        WatchEvent::State(WatchState::Unset) => Ok(None),
        WatchEvent::State(WatchState::Set(WatchValue::Path(value))) => {
            Ok(Some(LocusPath::new(value)))
        }
        WatchEvent::State(WatchState::Set(WatchValue::Property(_))) | WatchEvent::Change(_) => {
            read_relation(path).await
        }
    }
}

async fn read_relation(path: &Path) -> Result<Option<LocusPath>, String> {
    match locusfs_watch::read_link(path).await {
        Ok(value) => Ok(Some(LocusPath::new(value))),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(watch_error("read relation target", path, error)),
    }
}
