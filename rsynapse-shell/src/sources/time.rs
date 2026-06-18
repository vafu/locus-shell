use std::time::Duration;

use shell_core::{
    gtk::glib,
    source::{self, Observable, SourceError},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ClockView {
    pub(crate) time: String,
    pub(crate) date: String,
}

pub(crate) fn clock() -> Observable<ClockView> {
    source::from_async_loop(|emitter| async move {
        loop {
            match read_clock() {
                Ok(clock) => emitter.next(clock),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

fn read_clock() -> Result<ClockView, SourceError> {
    let now = glib::DateTime::now_local()
        .map_err(|error| SourceError::new(format!("failed to read local time: {error}")))?;
    let time = now
        .format("%H:%M")
        .map_err(|error| SourceError::new(format!("failed to format clock time: {error}")))?
        .to_string();
    let date = now
        .format("%a %b %d")
        .map_err(|error| SourceError::new(format!("failed to format clock date: {error}")))?
        .to_string();

    Ok(ClockView { time, date })
}
