use std::{
    collections::VecDeque,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use rxrust::prelude::Observable as _;

use super::{Observable, WatchEvent};

pub fn is_missing(error: &io::Error) -> bool {
    matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory)
}

pub fn watch_error(operation: &str, path: &Path, error: io::Error) -> String {
    format!("{operation} for {} failed: {error}", path.display())
}

pub fn log_errors<T>(
    source: &'static str,
    path: PathBuf,
    observable: Observable<T>,
) -> Observable<T>
where
    T: Send + 'static,
{
    observable
        .map_err(move |error| {
            eprintln!("[shell-core/source/{source}] {}: {error}", path.display());
            error
        })
        .box_it()
}

pub enum OpenedWatch {
    Target(locusfs_watch::Watch),
    Parent(locusfs_watch::Watch),
}

impl OpenedWatch {
    pub fn into_parts(self) -> (locusfs_watch::Watch, bool) {
        match self {
            Self::Target(watch) => (watch, true),
            Self::Parent(watch) => (watch, false),
        }
    }
}

pub async fn open_target_or_parent(path: &Path) -> Result<OpenedWatch, String> {
    match locusfs_watch::Watch::open(path).await {
        Ok(watch) => Ok(OpenedWatch::Target(watch)),
        Err(error) if is_missing(&error) => {
            let ancestor = nearest_watchable_ancestor(path)
                .ok_or_else(|| format!("path has no watchable ancestor: {}", path.display()))?;
            locusfs_watch::Watch::open(&ancestor)
                .await
                .map(OpenedWatch::Parent)
                .map_err(|error| watch_error("open ancestor watch", &ancestor, error))
        }
        Err(error) => Err(watch_error("open watch", path, error)),
    }
}

pub struct WatchEvents {
    pending: VecDeque<WatchEvent>,
}

impl WatchEvents {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    pub async fn next(&mut self, watch: &mut locusfs_watch::Watch) -> io::Result<WatchEvent> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }

        let raw = watch.wait_raw_event().await?;
        let text = std::str::from_utf8(&raw).map_err(|error| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("watch event is not valid UTF-8: {error}"),
            )
        })?;

        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            self.pending.push_back(WatchEvent::parse_text(line)?);
        }

        self.pending
            .pop_front()
            .ok_or_else(|| io::Error::new(ErrorKind::UnexpectedEof, "empty watch event payload"))
    }
}

fn nearest_watchable_ancestor(path: &Path) -> Option<PathBuf> {
    let mut ancestor = path.parent();
    while let Some(path) = ancestor {
        if path.exists() {
            return Some(path.to_owned());
        }
        ancestor = path.parent();
    }
    None
}
