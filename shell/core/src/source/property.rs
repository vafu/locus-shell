use std::path::{Path, PathBuf};

use futures_util::stream;
use rxrust::prelude::Observable as _;

use crate::source::Observable;

use super::{
    FromLocusValue, WatchEvent, WatchState, WatchValue,
    support::{
        WatchEvents, from_stream_result, is_missing, log_errors, open_target_or_parent,
        shared_source, watch_error,
    },
};

pub(super) fn property<T>(path: impl Into<PathBuf>) -> Observable<Option<T>>
where
    T: FromLocusValue,
{
    let path = path.into();
    shared_source("property", path, |path| {
        let observable = from_stream_result(property_stream::<T>(path.clone()))
            .distinct_until_changed()
            .box_it();
        log_errors("property", path, observable)
    })
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
    events: WatchEvents,
    watching_target: bool,
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
            events: WatchEvents::new(),
            watching_target: false,
            phase: PropertyStreamPhase::Open,
        },
        |mut state| async move {
            loop {
                match state.phase {
                    PropertyStreamPhase::Open => match open_target_or_parent(&state.path).await {
                        Ok(opened) => {
                            let (watch, watching_target) = opened.into_parts();
                            state.watch = Some(watch);
                            state.watching_target = watching_target;
                            state.phase = PropertyStreamPhase::InitialRead;
                        }
                        Err(error) => {
                            state.phase = PropertyStreamPhase::Done;
                            return Some((Err(error), state));
                        }
                    },
                    PropertyStreamPhase::InitialRead => {
                        state.phase = PropertyStreamPhase::WatchEvents;
                        let result = read_property::<T>(&state.path).await;
                        return Some((result, state));
                    }
                    PropertyStreamPhase::WatchEvents => {
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
                                    read_property(&state.path).await
                                } else {
                                    let unset =
                                        matches!(event, WatchEvent::State(WatchState::Unset));
                                    let result = property_event_value(&state.path, event).await;
                                    if property_watch_should_reopen(unset, &result) {
                                        state.watch = None;
                                        state.watching_target = false;
                                        state.phase = PropertyStreamPhase::Open;
                                    }
                                    result
                                }
                            }
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

fn property_watch_should_reopen<T>(unset_event: bool, result: &Result<Option<T>, String>) -> bool {
    unset_event || matches!(result, Ok(None))
}

#[cfg(test)]
mod tests {
    use super::property_watch_should_reopen;

    #[test]
    fn property_watch_reopens_on_explicit_unset() {
        assert!(property_watch_should_reopen(true, &Ok(Some(1))));
    }

    #[test]
    fn property_watch_reopens_when_event_read_finds_missing_property() {
        assert!(property_watch_should_reopen(
            false,
            &Ok::<_, String>(None::<i32>)
        ));
    }

    #[test]
    fn property_watch_stays_on_target_when_event_reads_value() {
        assert!(!property_watch_should_reopen(false, &Ok(Some(1))));
    }

    #[test]
    fn property_watch_error_does_not_mask_done_state() {
        assert!(!property_watch_should_reopen::<i32>(
            false,
            &Err("read failed".to_owned()),
        ));
    }
}
