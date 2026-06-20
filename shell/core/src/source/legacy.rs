#![allow(deprecated)]

use std::{
    future::Future,
    io::{self, ErrorKind},
    path::Path,
    pin::Pin,
};

use futures_util::future;

use crate::source::{Observable, from_async_loop};

use super::{LegacyWatchEvent, WatchSpec, WatchTarget};

type WatchFuture<'a> = Pin<Box<dyn Future<Output = io::Result<String>> + Send + 'a>>;

impl WatchSpec {
    async fn open(&self) -> io::Result<locusfs_watch::Watch> {
        open_watch(&self.path, self.target).await
    }
}

pub(super) fn change_events_async_impl<Open, OpenFuture>(
    mut open_watches: Open,
) -> Observable<LegacyWatchEvent>
where
    Open: FnMut() -> OpenFuture + Send + 'static,
    OpenFuture: Future<Output = Result<Vec<WatchSpec>, String>> + Send,
{
    from_async_loop(move |emitter| async move {
        let mut watches = Vec::new();
        let mut pending_event = LegacyWatchEvent::initial();

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
                emitter.error("no watches registered".to_owned());
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

pub(super) fn read_on_change_async_impl<Value, Read, ReadFuture>(
    watch: WatchSpec,
    mut read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, String>> + Send,
{
    from_async_loop(move |emitter| async move {
        let mut active_watch = match watch.open().await {
            Ok(watch) => watch,
            Err(error) => {
                emitter.error(format!("failed to watch {}: {error}", watch.path.display()));
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

            if let Err(error) = active_watch.wait_raw_event_to_string().await {
                emitter.error(format!(
                    "watch failed for {}: {error}",
                    watch.path.display()
                ));
                return;
            }
        }
    })
}

pub(super) fn read_on_any_change_async_impl<Value, Open, OpenFuture, Read, ReadFuture>(
    mut open_watches: Open,
    mut read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Open: FnMut() -> OpenFuture + Send + 'static,
    OpenFuture: Future<Output = Result<Vec<WatchSpec>, String>> + Send,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, String>> + Send,
{
    from_async_loop(move |emitter| async move {
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
                emitter.error("no watches registered".to_owned());
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
    watch: locusfs_watch::Watch,
}

async fn sync_watches(active: &mut Vec<ActiveWatch>, specs: &[WatchSpec]) -> Result<(), String> {
    if specs.is_empty() {
        return Err("no watches registered".to_owned());
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
                return Err(format!("failed to watch {}: {error}", spec.path.display()));
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

async fn wait_for_any_watch(watches: &mut [ActiveWatch]) -> Result<LegacyWatchEvent, String> {
    if watches.is_empty() {
        return Err("no watches registered".to_owned());
    }

    let paths = watches
        .iter()
        .map(|watch| watch.watch.data_path().to_path_buf())
        .collect::<Vec<_>>();
    let waiters = watches
        .iter_mut()
        .map(|watch| Box::pin(watch.watch.wait_raw_event_to_string()) as WatchFuture<'_>)
        .collect::<Vec<_>>();
    let (result, index, _) = future::select_all(waiters).await;
    result
        .map_err(|error| format!("watch failed for {}: {error}", paths[index].display()))
        .map(|event| LegacyWatchEvent::new(paths[index].clone(), event.trim().to_owned()))
}

async fn open_watch(path: &Path, target: WatchTarget) -> io::Result<locusfs_watch::Watch> {
    let data_path = locusfs_watch::absolute_path(path)?;
    let mount_root = locusfs_watch::find_mount_root(&data_path).await?;
    let mut logical_path = locusfs_watch::logical_watch_path(&mount_root, &data_path)?;

    if matches!(target, WatchTarget::Directory) && !logical_path.ends_with('/') {
        logical_path.push('/');
    }

    locusfs_watch::Watch::open_with_parts(data_path, mount_root, logical_path).await
}
