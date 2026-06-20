use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::{Observable as _, ObservableFactory as _, Shared};

use crate::source::Observable;

use super::{
    FromLocusValue, WatchEvent, WatchState, WatchValue,
    support::{is_missing, watch_error},
};

pub(super) fn property<T>(path: impl Into<PathBuf>) -> Observable<Option<T>>
where
    T: FromLocusValue,
{
    Shared::<()>::from_stream_result(property_stream(path.into()))
        .distinct_until_changed()
        .box_it()
}

enum PropertyStreamPhase {
    Open,
    InitialRead,
    WatchEvents,
    Done,
}

struct PropertyStreamState {
    path: PathBuf,
    watch: Option<locusfs_watch::Watch>,
    phase: PropertyStreamPhase,
}

fn property_stream<T>(path: PathBuf) -> impl futures_util::Stream<Item = Result<Option<T>, String>>
where
    T: FromLocusValue,
{
    stream::unfold(
        PropertyStreamState {
            path,
            watch: None,
            phase: PropertyStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    PropertyStreamPhase::Open => {
                        match locusfs_watch::Watch::open(&state.path).await {
                            Ok(watch) => {
                                state.watch = Some(watch);
                                state.phase = PropertyStreamPhase::InitialRead;
                            }
                            Err(error) => {
                                state.phase = PropertyStreamPhase::Done;
                                let error = watch_error("open property watch", &state.path, error);
                                return Some((Err(error), state));
                            }
                        }
                    }
                    PropertyStreamPhase::InitialRead => {
                        state.phase = PropertyStreamPhase::WatchEvents;
                        let result = read_property::<T>(&state.path).await;
                        return Some((result, state));
                    }
                    PropertyStreamPhase::WatchEvents => {
                        let result = match state
                            .watch
                            .as_mut()
                            .expect("watch initialized")
                            .next_event()
                            .await
                        {
                            Ok(event) => property_event_value(&state.path, event).await,
                            Err(error) => {
                                Err(watch_error("read property watch event", &state.path, error))
                            }
                        };

                        if result.is_err() {
                            state.phase = PropertyStreamPhase::Done;
                        }

                        return Some((result, state));
                    }
                    PropertyStreamPhase::Done => return None,
                }
            }
        },
    )
}

async fn property_event_value<T>(path: &Path, event: WatchEvent) -> Result<Option<T>, String>
where
    T: FromLocusValue,
{
    match event {
        WatchEvent::State(WatchState::Unset) => Ok(None),
        WatchEvent::State(WatchState::Set(WatchValue::Property(value))) => {
            decode_property(path, &value)
        }
        WatchEvent::State(WatchState::Set(WatchValue::Path(_))) | WatchEvent::Change(_) => {
            read_property(path).await
        }
    }
}

async fn read_property<T>(path: &Path) -> Result<Option<T>, String>
where
    T: FromLocusValue,
{
    match locusfs_watch::read_to_string(path).await {
        Ok(value) => decode_property(path, &value),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(watch_error("read property value", path, error)),
    }
}

fn decode_property<T>(path: &Path, value: &str) -> Result<Option<T>, String>
where
    T: FromLocusValue,
{
    T::from_locus_value(value.trim())
        .map(Some)
        .map_err(|error| format!("failed to decode property {}: {error}", path.display()))
}
