use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::Observable as _;

use crate::{locus_path::LocusPath, source::Observable};

use super::{
    WatchEvent, WatchState, WatchValue,
    support::{WatchEvents, from_stream_result, is_missing, log_errors, watch_error},
};

pub(super) fn relation(path: impl Into<PathBuf>) -> Observable<Option<LocusPath>> {
    let path = path.into();
    let observable = from_stream_result(relation_stream(path.clone()))
        .distinct_until_changed()
        .box_it();
    log_errors("relation", path, observable)
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
    events: WatchEvents,
    mount_root: Option<PathBuf>,
    phase: RelationStreamPhase,
}

fn relation_stream(
    path: PathBuf,
) -> impl futures_util::Stream<Item = Result<Option<LocusPath>, String>> {
    stream::unfold(
        RelationStreamState {
            path,
            watch: None,
            events: WatchEvents::new(),
            mount_root: None,
            phase: RelationStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    RelationStreamPhase::Open => {
                        match locusfs_watch::Watch::open(&state.path).await {
                            Ok(watch) => {
                                state.mount_root = Some(watch.mount_root().to_owned());
                                state.watch = Some(watch);
                                state.phase = RelationStreamPhase::InitialRead;
                            }
                            Err(error) => {
                                let error = watch_error("open relation watch", &state.path, error);
                                state.phase = RelationStreamPhase::Done;
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
                            .events
                            .next(state.watch.as_mut().expect("watch initialized"))
                            .await
                        {
                            Ok(event) => {
                                relation_event_value(
                                    &state.path,
                                    state.mount_root.as_ref().expect("mount root initialized"),
                                    event,
                                )
                                .await
                            }
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

async fn relation_event_value(
    path: &Path,
    mount_root: &Path,
    event: WatchEvent,
) -> Result<Option<LocusPath>, String> {
    match event {
        WatchEvent::State(WatchState::Unset) => Ok(None),
        WatchEvent::State(WatchState::Set(WatchValue::Path(path))) => {
            Ok(Some(watch_value_path(mount_root, &path)))
        }
        WatchEvent::State(WatchState::Set(WatchValue::Property(_))) | WatchEvent::Change(_) => {
            read_relation(path).await
        }
    }
}

fn watch_value_path(mount_root: &Path, value: &str) -> LocusPath {
    let path = Path::new(value);
    if path.is_absolute() {
        return LocusPath::new(mount_root.join(path.strip_prefix("/").unwrap_or(path)));
    }

    LocusPath::new(mount_root.join(path))
}

async fn read_relation(path: &Path) -> Result<Option<LocusPath>, String> {
    match locusfs_watch::read_link(path).await {
        Ok(value) => Ok(Some(LocusPath::new(value))),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(watch_error("read relation target", path, error)),
    }
}
