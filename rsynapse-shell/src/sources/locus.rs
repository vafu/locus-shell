use std::{
    fs,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
};

use futures_util::future;
use shell_core::source::{self, Observable, SourceError};

pub(crate) type NodeId = String;
pub(crate) type WorkspaceNode = String;
pub(crate) type WindowNode = String;

type WatchFuture<'a> = Pin<Box<dyn Future<Output = io::Result<String>> + Send + 'a>>;

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";

pub(crate) fn workspaces() -> Observable<Vec<WorkspaceNode>> {
    observe_with_refresh(WatchSpec::directory(workspaces_path()), read_workspaces)
}

pub(crate) fn selected_workspace_windows() -> Observable<Vec<WindowNode>> {
    observe_selected_workspace_windows()
}

pub(crate) fn workspace_index(workspace: WorkspaceNode) -> Observable<u32> {
    observe_property(node_property_path(&workspace, "index"), parse_u32)
}

pub(crate) fn workspace_name(workspace: WorkspaceNode) -> Observable<String> {
    observe_property(node_property_path(&workspace, "name"), parse_string)
}

pub(crate) fn workspace_urgent(workspace: WorkspaceNode) -> Observable<bool> {
    observe_property(node_property_path(&workspace, "urgent"), parse_bool)
}

pub(crate) fn workspace_active(workspace: WorkspaceNode) -> Observable<bool> {
    observe_property(node_property_path(&workspace, "selected"), parse_bool)
}

pub(crate) fn workspace_project_name(workspace: WorkspaceNode) -> Observable<Option<String>> {
    observe_workspace_project_property(workspace, &["display-name", "title", "name", "path"])
}

pub(crate) fn workspace_project_branch(workspace: WorkspaceNode) -> Observable<Option<String>> {
    observe_workspace_project_property(workspace, &["branch"])
}

pub(crate) fn workspace_project_icon(workspace: WorkspaceNode) -> Observable<Option<String>> {
    observe_workspace_project_property(workspace, &["display-icon", "icon"])
}

pub(crate) fn window_title(window: WindowNode) -> Observable<String> {
    observe_property(node_property_path(&window, "title"), parse_string)
}

pub(crate) fn window_app_id(window: WindowNode) -> Observable<String> {
    observe_property(node_property_path(&window, "app-id"), parse_string)
}

pub(crate) fn window_is_selected(window: WindowNode) -> Observable<bool> {
    observe_property(node_property_path(&window, "selected"), parse_bool)
}

fn observe_property<Value>(
    path: PathBuf,
    parse: fn(&str) -> Result<Value, SourceError>,
) -> Observable<Value>
where
    Value: Send + 'static,
{
    observe_with_refresh(WatchSpec::value(path.clone()), move || {
        let path = path.clone();
        async move { read_property(&path, parse).await }
    })
}

fn observe_workspace_project_property(
    workspace: WorkspaceNode,
    properties: &'static [&'static str],
) -> Observable<Option<String>> {
    source::from_async_loop(move |emitter| async move {
        let project_path = node_relation_path(&workspace, "project");
        let project_watch_path =
            locusfs_client::absolute_path(&project_path).unwrap_or_else(|_| project_path.clone());

        loop {
            let mut watches = match open_workspace_project_watches(&workspace, properties).await {
                Ok(watches) => watches,
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            };

            match read_workspace_project_property(&workspace, properties).await {
                Ok(value) => emitter.next(value),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            match wait_for_any_watch(&mut watches).await {
                Ok(update) => {
                    if update.path == project_watch_path && update.event.is_removed() {
                        emitter.next(None);
                    }
                }
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }
        }
    })
}

#[derive(Clone, Copy)]
enum WatchTarget {
    Value,
    Directory,
}

struct WatchSpec {
    path: PathBuf,
    target: WatchTarget,
}

impl WatchSpec {
    fn value(path: PathBuf) -> Self {
        Self {
            path,
            target: WatchTarget::Value,
        }
    }

    fn directory(path: PathBuf) -> Self {
        Self {
            path,
            target: WatchTarget::Directory,
        }
    }

    async fn open(&self) -> io::Result<locusfs_client::Watch> {
        let data_path = locusfs_client::absolute_path(&self.path)?;
        let mount_root = locusfs_client::find_mount_root(&data_path).await?;
        let mut logical_path = locusfs_client::logical_watch_path(&mount_root, &data_path)?;

        if matches!(self.target, WatchTarget::Directory) && !logical_path.ends_with('/') {
            logical_path.push('/');
        }

        locusfs_client::Watch::open_with_parts(data_path, mount_root, logical_path).await
    }
}

fn observe_with_refresh<Value, Read, ReadFuture>(
    watch: WatchSpec,
    mut read: Read,
) -> Observable<Value>
where
    Value: Send + 'static,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, SourceError>> + Send,
{
    source::from_async_loop(move |emitter| async move {
        loop {
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

            match read().await {
                Ok(value) => emitter.next(value),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            if let Err(error) = wait_for_watch(&mut active_watch).await {
                emitter.error(SourceError::new(format!(
                    "watch failed for {}: {error}",
                    watch.path.display()
                )));
                return;
            }
        }
    })
}

fn observe_selected_workspace_windows() -> Observable<Vec<WindowNode>> {
    source::from_async_loop(|emitter| async move {
        loop {
            let mut watches = match open_selected_workspace_window_watches().await {
                Ok(watches) => watches,
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            };

            match read_selected_workspace_windows().await {
                Ok(windows) => emitter.next(windows),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            if let Err(error) = wait_for_any_watch(&mut watches).await {
                emitter.error(error);
                return;
            }
        }
    })
}

async fn open_selected_workspace_window_watches() -> Result<Vec<locusfs_client::Watch>, SourceError>
{
    let mut watches = Vec::new();
    open_required_watch(&mut watches, WatchSpec::value(selected_workspace_path())).await?;
    open_required_watch(&mut watches, WatchSpec::directory(windows_path())).await?;

    for window in read_windows()? {
        open_optional_watch(
            &mut watches,
            WatchSpec::value(node_relation_path(&window, "workspace")),
        )
        .await?;
        open_optional_watch(
            &mut watches,
            WatchSpec::value(node_property_path(&window, "column")),
        )
        .await?;
        open_optional_watch(
            &mut watches,
            WatchSpec::value(node_property_path(&window, "row")),
        )
        .await?;
    }

    Ok(watches)
}

async fn open_workspace_project_watches(
    workspace: &str,
    properties: &[&str],
) -> Result<Vec<locusfs_client::Watch>, SourceError> {
    let mut watches = Vec::new();
    open_required_watch(
        &mut watches,
        WatchSpec::value(node_relation_path(workspace, "project")),
    )
    .await?;

    if let Ok(project) = read_link_node_id(&node_relation_path(workspace, "project")) {
        for property in properties {
            open_optional_watch(
                &mut watches,
                WatchSpec::value(node_property_path(&project, property)),
            )
            .await?;
        }
    }

    Ok(watches)
}

async fn open_required_watch(
    watches: &mut Vec<locusfs_client::Watch>,
    spec: WatchSpec,
) -> Result<(), SourceError> {
    let watch = spec.open().await.map_err(|error| {
        SourceError::new(format!("failed to watch {}: {error}", spec.path.display()))
    })?;
    watches.push(watch);
    Ok(())
}

async fn open_optional_watch(
    watches: &mut Vec<locusfs_client::Watch>,
    spec: WatchSpec,
) -> Result<(), SourceError> {
    match spec.open().await {
        Ok(watch) => {
            watches.push(watch);
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(SourceError::new(format!(
            "failed to watch {}: {error}",
            spec.path.display()
        ))),
    }
}

struct WatchUpdate {
    path: PathBuf,
    event: WatchEvent,
}

async fn wait_for_any_watch(
    watches: &mut [locusfs_client::Watch],
) -> Result<WatchUpdate, SourceError> {
    if watches.is_empty() {
        return Err(SourceError::new("no watches registered"));
    }

    let paths = watches
        .iter()
        .map(|watch| watch.data_path().to_path_buf())
        .collect::<Vec<_>>();
    let waiters = watches
        .iter_mut()
        .map(|watch| Box::pin(wait_for_watch(watch)) as WatchFuture<'_>)
        .collect::<Vec<_>>();
    let (result, index, _) = future::select_all(waiters).await;
    let event = result.map_err(|error| {
        SourceError::new(format!(
            "watch failed for {}: {error}",
            paths[index].display()
        ))
    })?;
    Ok(WatchUpdate {
        path: paths[index].clone(),
        event: parse_watch_event(&event),
    })
}

async fn wait_for_watch(watch: &mut locusfs_client::Watch) -> io::Result<String> {
    watch.wait_event_to_string().await
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum WatchEvent {
    Added,
    Change,
    Removed,
    Updated,
    NodeAdded(NodeId),
    NodeChanged(NodeId),
    NodeRemoved(NodeId),
    Raw(String),
}

fn parse_watch_event(event: &str) -> WatchEvent {
    let event = event.trim();
    match event {
        "added" | "created" => WatchEvent::Added,
        "change" => WatchEvent::Change,
        "removed" => WatchEvent::Removed,
        "updated" => WatchEvent::Updated,
        _ => parse_node_event(event).unwrap_or_else(|| WatchEvent::Raw(event.to_owned())),
    }
}

impl WatchEvent {
    fn is_removed(&self) -> bool {
        matches!(self, Self::Removed | Self::NodeRemoved(_))
    }
}

fn parse_node_event(event: &str) -> Option<WatchEvent> {
    let (prefix, node) = event.rsplit_once(' ')?;
    let node = node.to_owned();
    match prefix {
        "node added" => Some(WatchEvent::NodeAdded(node)),
        "node changed" => Some(WatchEvent::NodeChanged(node)),
        "node removed" => Some(WatchEvent::NodeRemoved(node)),
        _ => None,
    }
}

async fn read_workspaces() -> Result<Vec<WorkspaceNode>, SourceError> {
    let mut workspaces = fs::read_dir(workspaces_path())
        .map_err(|error| SourceError::new(format!("failed to read workspaces: {error}")))?
        .map(|entry| {
            let entry = entry.map_err(|error| SourceError::new(error.to_string()))?;
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_| SourceError::new("workspace id is not valid UTF-8"))?;
            Ok(format!("workspace:{id}"))
        })
        .collect::<Result<Vec<_>, SourceError>>()?;

    let mut indexed = Vec::with_capacity(workspaces.len());
    for workspace in workspaces.drain(..) {
        let index = read_property(&node_property_path(&workspace, "index"), parse_u32)
            .await
            .unwrap_or(u32::MAX);
        indexed.push((index, workspace));
    }
    indexed.sort_by_key(|(index, _)| *index);
    workspaces = indexed
        .into_iter()
        .map(|(_, workspace)| workspace)
        .collect();
    Ok(workspaces)
}

fn read_windows() -> Result<Vec<WindowNode>, SourceError> {
    fs::read_dir(windows_path())
        .map_err(|error| SourceError::new(format!("failed to read windows: {error}")))?
        .map(|entry| {
            let entry = entry.map_err(|error| SourceError::new(error.to_string()))?;
            let window_id = entry
                .file_name()
                .into_string()
                .map_err(|_| SourceError::new("window id is not valid UTF-8"))?;
            Ok(format!("window:{window_id}"))
        })
        .collect()
}

async fn read_selected_workspace_windows() -> Result<Vec<WindowNode>, SourceError> {
    let selected = read_link_node_id(&selected_workspace_path())?;
    let mut windows = read_windows()?
        .into_iter()
        .filter_map(|node| {
            let workspace = read_window_workspace(&node).ok()?;
            (workspace == selected).then_some(node)
        })
        .collect::<Vec<_>>();

    let mut indexed = Vec::with_capacity(windows.len());
    for window in windows.drain(..) {
        let key = read_window_sort_key(&window).await;
        indexed.push((key, window));
    }
    indexed.sort_by_key(|(index, _)| *index);
    windows = indexed.into_iter().map(|(_, window)| window).collect();
    Ok(windows)
}

async fn read_window_sort_key(window: &str) -> (u32, u32, u32) {
    let column = read_property(&node_property_path(window, "column"), parse_u32)
        .await
        .unwrap_or(u32::MAX);
    let row = read_property(&node_property_path(window, "row"), parse_u32)
        .await
        .unwrap_or(u32::MAX);
    let id = read_property(&node_property_path(window, "id"), parse_u32)
        .await
        .unwrap_or(u32::MAX);
    (column, row, id)
}

async fn read_workspace_project_property(
    workspace: &str,
    properties: &[&str],
) -> Result<Option<String>, SourceError> {
    let project = match read_link_node_id(&node_relation_path(workspace, "project")) {
        Ok(project) => project,
        Err(_) => return Ok(None),
    };

    for property in properties {
        let path = node_property_path(&project, property);
        match read_property(&path, parse_string).await {
            Ok(value) => {
                if let Some(value) = optional_text(&value) {
                    return Ok(Some(value.to_owned()));
                }
            }
            Err(_) => continue,
        }
    }

    Ok(None)
}

fn read_window_workspace(window: &str) -> Result<NodeId, SourceError> {
    read_link_node_id(&node_relation_path(window, "workspace")).or_else(|_| {
        let workspace_id =
            fs::read_to_string(node_property_path(window, "workspace-id")).map_err(|error| {
                SourceError::new(format!("failed to read {window} workspace: {error}"))
            })?;
        Ok(format!("workspace:{}", workspace_id.trim()))
    })
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

fn optional_text(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn selected_workspace_path() -> PathBuf {
    root().join("context/selected/workspace")
}

fn workspaces_path() -> PathBuf {
    root().join("workspace")
}

fn windows_path() -> PathBuf {
    root().join("window")
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
