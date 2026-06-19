use std::{
    future::Future,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    pin::Pin,
};

use futures_util::future;
use shell_core::source::{self, Observable, SourceError};

type WatchFuture<'a> = Pin<Box<dyn Future<Output = io::Result<String>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WatchEvent {
    path: Option<PathBuf>,
    message: String,
}

impl WatchEvent {
    fn initial() -> Self {
        Self {
            path: None,
            message: "initial".to_owned(),
        }
    }

    fn new(path: PathBuf, message: String) -> Self {
        Self {
            path: Some(path),
            message,
        }
    }

    pub(crate) fn is_initial(&self) -> bool {
        self.path.is_none()
    }

    pub(crate) fn is_unset(&self) -> bool {
        self.event_name() == Some("unset")
    }

    pub(crate) fn is_set(&self) -> bool {
        self.event_name() == Some("set")
    }

    pub(crate) fn resolved_path(&self) -> Option<&Path> {
        let path = self
            .message
            .strip_prefix("set ")
            .filter(|path| !path.is_empty())?;
        Some(Path::new(path))
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn event_name(&self) -> Option<&str> {
        self.message.split_whitespace().next()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WatchTarget {
    Value,
    Directory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WatchSpec {
    path: PathBuf,
    target: WatchTarget,
    required: bool,
}

impl WatchSpec {
    pub(crate) fn value(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Value,
            required: true,
        }
    }

    pub(crate) fn directory(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Directory,
            required: true,
        }
    }

    pub(crate) fn optional_value(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Value,
            required: false,
        }
    }

    pub(crate) fn optional_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            target: WatchTarget::Directory,
            required: false,
        }
    }

    async fn open(&self) -> io::Result<locusfs_client::Watch> {
        open_watch(&self.path, self.target).await
    }
}

pub(crate) fn change_events_async<Open, OpenFuture>(
    mut open_watches: Open,
) -> Observable<WatchEvent>
where
    Open: FnMut() -> OpenFuture + Send + 'static,
    OpenFuture: Future<Output = Result<Vec<WatchSpec>, SourceError>> + Send,
{
    source::from_async_loop(move |emitter| async move {
        let mut watches = Vec::new();
        let mut pending_event = WatchEvent::initial();

        loop {
            let specs = match open_watches().await {
                Ok(specs) => specs,
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            };

            if let Err(error) = sync_watches(&mut watches, &specs).await {
                emitter.error(error);
                return;
            }

            if watches.is_empty() {
                emitter.error(SourceError::new("no watches registered"));
                return;
            }

            emitter.next(pending_event);

            pending_event = match wait_for_any_watch(&mut watches).await {
                Ok(event) => event,
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            };
        }
    })
}

pub(crate) fn read_on_change_async<Value, Read, ReadFuture>(
    watch: WatchSpec,
    mut read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, SourceError>> + Send,
{
    source::from_async_loop(move |emitter| async move {
        let mut active_watch = match watch.open().await {
            Ok(watch) => watch,
            Err(error) => {
                emitter.error(SourceError::new(format!(
                    "failed to watch {}: {error}",
                    watch.path.display()
                )));
                return;
            }
        };

        let mut has_value = false;
        loop {
            match read().await {
                Ok(value) => {
                    has_value = true;
                    emitter.next(value);
                }
                Err(error) => {
                    if !has_value {
                        emitter.error(error);
                        return;
                    }
                }
            }

            if let Err(error) = active_watch.wait_event_to_string().await {
                emitter.error(SourceError::new(format!(
                    "watch failed for {}: {error}",
                    watch.path.display()
                )));
                return;
            }
        }
    })
}

pub(crate) fn read_on_any_change_async<Value, Open, OpenFuture, Read, ReadFuture>(
    mut open_watches: Open,
    mut read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Open: FnMut() -> OpenFuture + Send + 'static,
    OpenFuture: Future<Output = Result<Vec<WatchSpec>, SourceError>> + Send,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, SourceError>> + Send,
{
    source::from_async_loop(move |emitter| async move {
        let mut watches = Vec::new();
        let mut has_value = false;

        loop {
            let specs = match open_watches().await {
                Ok(specs) => specs,
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            };

            if let Err(error) = sync_watches(&mut watches, &specs).await {
                emitter.error(error);
                return;
            }

            if watches.is_empty() {
                emitter.error(SourceError::new("no watches registered"));
                return;
            }

            match read().await {
                Ok(value) => {
                    has_value = true;
                    emitter.next(value);
                }
                Err(error) => {
                    if !has_value {
                        emitter.error(error);
                        return;
                    }
                }
            };

            if let Err(error) = wait_for_any_watch(&mut watches).await {
                emitter.error(error);
                return;
            }
        }
    })
}

struct ActiveWatch {
    spec: WatchSpec,
    watch: locusfs_client::Watch,
}

async fn sync_watches(
    active: &mut Vec<ActiveWatch>,
    specs: &[WatchSpec],
) -> Result<(), SourceError> {
    if specs.is_empty() {
        return Err(SourceError::new("no watches registered"));
    }

    let mut next = Vec::with_capacity(specs.len());
    for spec in specs {
        if let Some(index) = active.iter().position(|active| active.spec == *spec) {
            next.push(active.remove(index));
            continue;
        }

        match open_active_watch(spec).await {
            Ok(watch) => next.push(watch),
            Err(error) if !spec.required && error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(SourceError::new(format!(
                    "failed to watch {}: {error}",
                    spec.path.display()
                )));
            }
        }
    }

    *active = next;
    Ok(())
}

async fn open_active_watch(spec: &WatchSpec) -> io::Result<ActiveWatch> {
    spec.open().await.map(|watch| ActiveWatch {
        spec: spec.clone(),
        watch,
    })
}

async fn wait_for_any_watch(watches: &mut [ActiveWatch]) -> Result<WatchEvent, SourceError> {
    if watches.is_empty() {
        return Err(SourceError::new("no watches registered"));
    }

    let paths = watches
        .iter()
        .map(|watch| watch.watch.data_path().to_path_buf())
        .collect::<Vec<_>>();
    let waiters = watches
        .iter_mut()
        .map(|watch| Box::pin(watch.watch.wait_event_to_string()) as WatchFuture<'_>)
        .collect::<Vec<_>>();
    let (result, index, _) = future::select_all(waiters).await;
    result
        .map_err(|error| {
            SourceError::new(format!(
                "watch failed for {}: {error}",
                paths[index].display()
            ))
        })
        .map(|event| WatchEvent::new(paths[index].clone(), event.trim().to_owned()))
}

async fn open_watch(path: &Path, target: WatchTarget) -> io::Result<locusfs_client::Watch> {
    let data_path = locusfs_client::absolute_path(path)?;
    let mount_root = locusfs_client::find_mount_root(&data_path).await?;
    let mut logical_path = locusfs_client::logical_watch_path(&mount_root, &data_path)?;

    if matches!(target, WatchTarget::Directory) && !logical_path.ends_with('/') {
        logical_path.push('/');
    }

    locusfs_client::Watch::open_with_parts(data_path, mount_root, logical_path).await
}
