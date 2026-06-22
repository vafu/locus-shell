use std::path::PathBuf;

use futures_util::stream;
use rxrust::prelude::{Observable as _, ObservableFactory as _, Shared};

use crate::source::Observable;

use super::{
    WatchEvent,
    support::{WatchEvents, log_errors, watch_error},
};

pub(super) fn watch(path: impl Into<PathBuf>) -> Observable<locusfs_watch::WatchEvent> {
    let path = path.into();
    let observable = Shared::<()>::from_stream_result(watch_event_stream(path.clone())).box_it();
    log_errors("watch", path, observable)
}

struct WatchEventStreamState {
    path: PathBuf,
    watch: Option<locusfs_watch::Watch>,
    events: WatchEvents,
    done: bool,
}

fn watch_event_stream(
    path: PathBuf,
) -> impl futures_util::Stream<Item = Result<WatchEvent, String>> {
    stream::unfold(
        WatchEventStreamState {
            path,
            watch: None,
            events: WatchEvents::new(),
            done: false,
        },
        |mut state| async move {
            if state.done {
                return None;
            }

            if state.watch.is_none() {
                match locusfs_watch::Watch::open(&state.path).await {
                    Ok(watch) => state.watch = Some(watch),
                    Err(error) => {
                        state.done = true;
                        let error = watch_error("open watch", &state.path, error);
                        return Some((Err(error), state));
                    }
                }
            }

            let result = state
                .events
                .next(state.watch.as_mut().expect("watch initialized"))
                .await
                .map_err(|error| watch_error("read watch event", &state.path, error));

            if result.is_err() {
                state.done = true;
            }

            Some((result, state))
        },
    )
}
