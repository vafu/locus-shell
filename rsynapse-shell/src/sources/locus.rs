use std::{
    future::Future,
    path::{Path, PathBuf},
};

use shell_core::source::{self, LegacyWatchEvent, Observable, WatchSpec, rx::Observable as _};

pub(crate) type NodeId = String;
pub(crate) type WorkspaceNode = String;
pub(crate) type WindowNode = String;

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

fn observe_property<Value>(
    path: PathBuf,
    parse: fn(&str) -> Result<Value, String>,
) -> Observable<Value>
where
    Value: Send + PartialEq + Clone + 'static,
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
    source::read_on_any_change_async(
        {
            let workspace = workspace.clone();
            move || {
                let workspace = workspace.clone();
                async move { open_workspace_project_watches(&workspace, properties).await }
            }
        },
        move || {
            let workspace = workspace.clone();
            async move { read_workspace_project_property(&workspace, properties).await }
        },
    )
    .distinct_until_changed()
    .box_it()
}

fn observe_with_refresh<Value, Read, ReadFuture>(watch: WatchSpec, read: Read) -> Observable<Value>
where
    Value: Send + PartialEq + Clone + 'static,
    Read: FnMut() -> ReadFuture + Send + 'static,
    ReadFuture: Future<Output = Result<Value, String>> + Send,
{
    source::read_on_change_async(watch, read)
        .distinct_until_changed()
        .box_it()
}

fn observe_selected_workspace_windows() -> Observable<Vec<WindowNode>> {
    let mut selected_workspace = SelectedWorkspaceState::Unknown;
    source::change_events_async(open_selected_workspace_window_watches)
        .filter_map(move |event| selected_workspace_windows_refresh(event, &mut selected_workspace))
        .switch_map(read_selected_workspace_windows_once)
        .filter_map(|windows| windows)
        .distinct_until_changed()
        .box_it()
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SelectedWorkspaceState {
    Unknown,
    Unset,
    Set(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SelectedWorkspaceRead {
    ResolveCurrent,
    Cached(Option<NodeId>),
}

fn selected_workspace_windows_refresh(
    event: LegacyWatchEvent,
    selected_workspace: &mut SelectedWorkspaceState,
) -> Option<SelectedWorkspaceRead> {
    if event.is_initial() {
        return Some(selected_workspace_read(selected_workspace));
    }
    if event.is_unset() {
        if event.path() == Some(selected_workspace_path().as_path()) {
            *selected_workspace = SelectedWorkspaceState::Unset;
            return Some(SelectedWorkspaceRead::Cached(None));
        }
        return Some(selected_workspace_read(selected_workspace));
    }
    if event.is_set() {
        if event.path() != Some(selected_workspace_path().as_path()) {
            return Some(selected_workspace_read(selected_workspace));
        }
        let selected = event
            .resolved_path()
            .map(node_id_from_path)
            .transpose()
            .ok()
            .flatten()?;
        *selected_workspace = SelectedWorkspaceState::Set(selected.clone());
        return Some(SelectedWorkspaceRead::Cached(Some(selected)));
    }
    Some(selected_workspace_read(selected_workspace))
}

fn selected_workspace_read(selected_workspace: &SelectedWorkspaceState) -> SelectedWorkspaceRead {
    match selected_workspace {
        SelectedWorkspaceState::Unknown => SelectedWorkspaceRead::ResolveCurrent,
        SelectedWorkspaceState::Unset => SelectedWorkspaceRead::Cached(None),
        SelectedWorkspaceState::Set(selected) => {
            SelectedWorkspaceRead::Cached(Some(selected.clone()))
        }
    }
}

fn read_selected_workspace_windows_once(
    selected: SelectedWorkspaceRead,
) -> Observable<Option<Vec<WindowNode>>> {
    #[allow(deprecated)]
    source::from_async_loop(|emitter| async move {
        emitter.next(read_selected_workspace_windows_async(selected).await.ok());
        emitter.complete();
    })
}

async fn open_selected_workspace_window_watches() -> Result<Vec<WatchSpec>, String> {
    let mut watches = vec![
        WatchSpec::value(selected_workspace_path()),
        WatchSpec::directory(windows_path()),
    ];

    let Ok(windows) = read_windows().await else {
        return Ok(watches);
    };

    for window in windows {
        watches.push(WatchSpec::optional_value(node_relation_path(
            &window,
            "workspace",
        )));
        push_window_sort_watches(&mut watches, &window).await;
    }

    Ok(watches)
}

async fn push_window_sort_watches(watches: &mut Vec<WatchSpec>, window: &str) {
    let column = node_property_path(window, "column");
    if locusfs_watch::exists(&column).await {
        watches.push(WatchSpec::optional_value(column));
    }
    let row = node_property_path(window, "row");
    if locusfs_watch::exists(&row).await {
        watches.push(WatchSpec::optional_value(row));
    }
}

async fn open_workspace_project_watches(
    workspace: &str,
    properties: &[&str],
) -> Result<Vec<WatchSpec>, String> {
    let mut watches = vec![WatchSpec::value(node_relation_path(workspace, "project"))];

    if let Ok(project) = read_link_node_id(&node_relation_path(workspace, "project")).await {
        for property in properties {
            let path = node_property_path(&project, property);
            if locusfs_watch::exists(&path).await {
                watches.push(WatchSpec::optional_value(path));
            }
        }
    }

    Ok(watches)
}

async fn read_workspaces() -> Result<Vec<WorkspaceNode>, String> {
    let mut workspaces = locusfs_watch::read_dir_names(workspaces_path())
        .await
        .map_err(|error| format!("failed to read workspaces: {error}"))?
        .into_iter()
        .map(|id| format!("workspace:{id}"))
        .collect::<Vec<_>>();

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

async fn read_windows() -> Result<Vec<WindowNode>, String> {
    Ok(locusfs_watch::read_dir_names(windows_path())
        .await
        .map_err(|error| format!("failed to read windows: {error}"))?
        .into_iter()
        .map(|window_id| format!("window:{window_id}"))
        .collect())
}

async fn read_selected_workspace_windows_async(
    selected: SelectedWorkspaceRead,
) -> Result<Vec<WindowNode>, String> {
    let selected = match selected {
        SelectedWorkspaceRead::Cached(Some(selected)) => selected,
        SelectedWorkspaceRead::Cached(None) => return Ok(Vec::new()),
        SelectedWorkspaceRead::ResolveCurrent => read_link_node_id(&selected_workspace_path())
            .await
            .map_err(|error| {
                format!(
                    "selected workspace windows read failed: failed to resolve selected workspace: {error}"
                )
            })?,
    };
    let mut windows = Vec::new();
    for node in read_windows().await? {
        let Ok(workspace) = read_window_workspace(&node).await else {
            continue;
        };
        if workspace == selected {
            windows.push(node);
        }
    }

    sort_windows_async(windows).await
}

async fn sort_windows_async(mut windows: Vec<WindowNode>) -> Result<Vec<WindowNode>, String> {
    let mut indexed = Vec::with_capacity(windows.len());
    for window in windows.drain(..) {
        let key = read_window_sort_key_async(&window).await;
        indexed.push((key, window));
    }
    indexed.sort_by_key(|(index, _)| *index);
    Ok(indexed.into_iter().map(|(_, window)| window).collect())
}

async fn read_window_sort_key_async(window: &str) -> (u32, u32, u32) {
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
) -> Result<Option<String>, String> {
    let project = match read_link_node_id(&node_relation_path(workspace, "project")).await {
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

async fn read_window_workspace(window: &str) -> Result<NodeId, String> {
    if let Ok(workspace) = read_link_node_id(&node_relation_path(window, "workspace")).await {
        return Ok(workspace);
    }

    let workspace_id = read_property(&node_property_path(window, "workspace-id"), parse_string)
        .await
        .map_err(|error| format!("failed to read {window} workspace: {error}"))?;
    Ok(format!("workspace:{workspace_id}"))
}

fn node_id_from_path(path: &Path) -> Result<NodeId, String> {
    let local = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid node path: {}", path.display()))?;
    let kind = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid node path: {}", path.display()))?;

    Ok(format!("{kind}:{local}"))
}

async fn read_property<Value>(
    path: &Path,
    parse: fn(&str) -> Result<Value, String>,
) -> Result<Value, String> {
    let value = locusfs_watch::read_to_string(path)
        .await
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    parse(value.trim())
}

async fn read_link_node_id(path: &Path) -> Result<NodeId, String> {
    let path = locusfs_watch::read_link(path)
        .await
        .map_err(|error| format!("failed to resolve {}: {error}", path.display()))?;

    let local = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid node path: {}", path.display()))?;
    let kind = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid node path: {}", path.display()))?;

    Ok(format!("{kind}:{local}"))
}

fn parse_string(value: &str) -> Result<String, String> {
    Ok(value.to_owned())
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(format!("invalid bool value: {value}")),
    }
}

fn parse_u32(value: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|error| format!("invalid u32 value {value}: {error}"))
}

fn optional_text(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn selected_workspace_path() -> PathBuf {
    selected_context_path().join("workspace")
}

fn selected_context_path() -> PathBuf {
    root().join("context/selected")
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
