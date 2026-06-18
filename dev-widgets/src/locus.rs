use std::{
    fs,
    future::Future,
    path::{Path, PathBuf},
};

use shell_core::source::{self, Observable, SourceError};

pub(super) type NodeId = String;
pub(super) type WindowNode = String;

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/locusfs-niri-run";

pub(super) fn selected_workspace_windows() -> Observable<Vec<WindowNode>> {
    observe_with_refresh(selected_workspace_path(), read_selected_workspace_windows)
}

pub(super) fn window_title(window: WindowNode) -> Observable<String> {
    observe_property(node_property_path(&window, "title"), parse_string)
}

pub(super) fn window_is_selected(window: WindowNode) -> Observable<bool> {
    observe_property(node_property_path(&window, "selected"), parse_bool)
}

fn observe_property<Value>(
    path: PathBuf,
    parse: fn(&str) -> Result<Value, SourceError>,
) -> Observable<Value>
where
    Value: Send + 'static,
{
    observe_with_refresh(path.clone(), move || {
        let path = path.clone();
        async move { read_property(&path, parse).await }
    })
}

fn observe_with_refresh<Value, Read, ReadFuture>(
    watch_path: PathBuf,
    mut read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, SourceError>> + Send,
{
    source::from_async_loop(move |emitter| async move {
        loop {
            let mut watch = match locusfs_client::Watch::open(&watch_path).await {
                Ok(watch) => watch,
                Err(error) => {
                    emitter.error(SourceError::new(format!(
                        "failed to watch {}: {error}",
                        watch_path.display()
                    )));
                    return;
                }
            };

            match read().await {
                Ok(value) => emitter.next(value),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            if let Err(error) = watch.wait().await {
                emitter.error(SourceError::new(format!(
                    "watch failed for {}: {error}",
                    watch_path.display()
                )));
                return;
            }
        }
    })
}

async fn read_selected_workspace_windows() -> Result<Vec<WindowNode>, SourceError> {
    let selected = read_link_node_id(&selected_workspace_path())?;
    let mut windows = fs::read_dir(root().join("window"))
        .map_err(|error| SourceError::new(format!("failed to read windows: {error}")))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let window_id = entry.file_name().into_string().ok()?;
            let node = format!("window:{window_id}");
            let workspace = read_link_node_id(&node_relation_path(&node, "workspace")).ok()?;
            (workspace == selected).then_some(node)
        })
        .collect::<Vec<_>>();

    let mut indexed = Vec::with_capacity(windows.len());
    for window in windows.drain(..) {
        let index = read_property(&node_property_path(&window, "id"), parse_u32)
            .await
            .unwrap_or(u32::MAX);
        indexed.push((index, window));
    }
    indexed.sort_by_key(|(index, _)| *index);
    Ok(indexed.into_iter().map(|(_, window)| window).collect())
}

async fn read_property<Value>(
    path: &Path,
    parse: fn(&str) -> Result<Value, SourceError>,
) -> Result<Value, SourceError> {
    let value = locusfs_client::read_to_string(path)
        .await
        .map_err(|error| SourceError::new(format!("failed to read {}: {error}", path.display())))?;
    parse(value.trim())
}

fn read_link_node_id(path: &Path) -> Result<NodeId, SourceError> {
    let target = fs::read_link(path).map_err(|error| {
        SourceError::new(format!("failed to resolve {}: {error}", path.display()))
    })?;
    let path = if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or_else(|| Path::new("/")).join(target)
    };

    let local = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| SourceError::new(format!("invalid node path: {}", path.display())))?;
    let kind = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| SourceError::new(format!("invalid node path: {}", path.display())))?;

    Ok(format!("{kind}:{local}"))
}

fn parse_string(value: &str) -> Result<String, SourceError> {
    Ok(value.to_owned())
}

fn parse_bool(value: &str) -> Result<bool, SourceError> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(SourceError::new(format!("invalid bool value: {value}"))),
    }
}

fn parse_u32(value: &str) -> Result<u32, SourceError> {
    value
        .parse()
        .map_err(|error| SourceError::new(format!("invalid u32 value {value}: {error}")))
}

fn selected_workspace_path() -> PathBuf {
    root().join("context/selected/workspace")
}

fn node_property_path(node: &str, property: &str) -> PathBuf {
    node_path(node).join(property)
}

fn node_relation_path(node: &str, relation: &str) -> PathBuf {
    node_path(node).join(relation)
}

fn node_path(node: &str) -> PathBuf {
    let (kind, local) = node.split_once(':').unwrap_or(("node", node));
    root().join(kind).join(local)
}

fn root() -> PathBuf {
    std::env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
}
