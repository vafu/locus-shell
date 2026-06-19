use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use shell_core::source::{Observable, SourceError, rx::Observable as _};

use crate::sources::{
    WindowNode,
    watch::{self, WatchSpec},
};

use super::{WindowTileKind, WindowTileView};

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";

pub(crate) fn window_tile_for_window(window: WindowNode) -> Observable<WindowTileView> {
    watch::read_on_any_change_async(
        {
            let window = window.clone();
            move || {
                let window = window.clone();
                async move { open_window_tile_watches(&window).await }
            }
        },
        move || {
            let window = window.clone();
            async move { read_window_tile(&window).await }
        },
    )
    .distinct_until_changed()
    .box_it()
}

async fn open_window_tile_watches(window: &str) -> Result<Vec<WatchSpec>, SourceError> {
    let mut watches = vec![WatchSpec::directory(node_path(window))];
    push_optional_values(
        &mut watches,
        window,
        &["title", "app-id", "urgent", "app-instance"],
    );

    let Some(app_instance) = read_link_node_id(&node_relation_path(window, "app-instance"))
        .await
        .ok()
    else {
        return Ok(watches);
    };
    watches.push(WatchSpec::optional_directory(node_path(&app_instance)));
    push_optional_values(
        &mut watches,
        &app_instance,
        &["name", "icon", "agent-session"],
    );

    let Some(agent_session) =
        read_link_node_id(&node_relation_path(&app_instance, "agent-session"))
            .await
            .ok()
    else {
        return Ok(watches);
    };
    watches.push(WatchSpec::optional_directory(node_path(&agent_session)));
    push_optional_values(
        &mut watches,
        &agent_session,
        &[
            "agent",
            "agent_nickname",
            "context_pct",
            "cwd",
            "model",
            "raw_title",
            "requires_attention",
            "session-project",
            "state",
            "task_complete",
        ],
    );
    watches.push(WatchSpec::optional_directory(node_relation_path(
        &agent_session,
        "subagent-session",
    )));

    if let Some(project) = read_link_node_id(&node_relation_path(&agent_session, "session-project"))
        .await
        .ok()
    {
        watches.push(WatchSpec::optional_directory(node_path(&project)));
        push_optional_values(&mut watches, &project, &["display-icon", "icon", "name"]);
    }

    Ok(watches)
}

fn push_optional_values(watches: &mut Vec<WatchSpec>, node: &str, properties: &[&str]) {
    watches.extend(
        properties
            .iter()
            .map(|property| WatchSpec::optional_value(node_property_path(node, property))),
    );
}

async fn read_window_tile(window: &str) -> Result<WindowTileView, SourceError> {
    let title = read_optional_text(&node_property_path(window, "title"))
        .await
        .unwrap_or_default();
    let app_id = read_optional_text(&node_property_path(window, "app-id"))
        .await
        .unwrap_or_default();
    let window_urgent = read_bool(&node_property_path(window, "urgent"))
        .await
        .unwrap_or(false);
    let icon = icon_for_app_id(&app_id);

    if let Some(app_instance) = read_link_node_id(&node_relation_path(window, "app-instance"))
        .await
        .ok()
        && let Some(agent_session) =
            read_link_node_id(&node_relation_path(&app_instance, "agent-session"))
                .await
                .ok()
    {
        return agent_window_tile(&title, &icon, window_urgent, &app_instance, &agent_session)
            .await;
    }

    Ok(WindowTileView {
        kind: window_kind(&title),
        icon,
        tooltip: title,
        urgent: window_urgent,
        context_pct: 0,
        substatus_count: 0,
    })
}

async fn agent_window_tile(
    title: &str,
    window_icon: &str,
    window_urgent: bool,
    app_instance: &str,
    agent_session: &str,
) -> Result<WindowTileView, SourceError> {
    let app_icon = read_optional_text(&node_property_path(app_instance, "icon")).await;
    let app_name = read_optional_text(&node_property_path(app_instance, "name")).await;
    let agent = read_optional_text(&node_property_path(agent_session, "agent")).await;
    let nickname = read_optional_text(&node_property_path(agent_session, "agent_nickname")).await;
    let model = read_optional_text(&node_property_path(agent_session, "model")).await;
    let state = read_optional_text(&node_property_path(agent_session, "state")).await;
    let cwd = read_optional_text(&node_property_path(agent_session, "cwd")).await;
    let raw_title = read_optional_text(&node_property_path(agent_session, "raw_title")).await;
    let requires_attention = read_bool(&node_property_path(agent_session, "requires_attention"))
        .await
        .unwrap_or(false);
    let context_pct = read_context_pct(&node_property_path(agent_session, "context_pct")).await;

    let icon = read_project_icon(agent_session)
        .await
        .or(app_icon)
        .filter(|icon| !icon.is_empty())
        .unwrap_or_else(|| {
            if window_icon.is_empty() {
                "smart_toy".to_owned()
            } else {
                window_icon.to_owned()
            }
        });

    Ok(WindowTileView {
        kind: WindowTileKind::Agent,
        icon,
        tooltip: agent_tooltip(AgentTooltip {
            title: first_non_empty(&[raw_title.as_deref(), Some(title)]),
            agent: first_non_empty(&[nickname.as_deref(), agent.as_deref(), app_name.as_deref()]),
            model: model.as_deref(),
            state: state.as_deref(),
            cwd: cwd.as_deref(),
            context_pct,
        }),
        urgent: window_urgent || requires_attention,
        context_pct,
        substatus_count: count_subagent_sessions(agent_session).await,
    })
}

struct AgentTooltip<'a> {
    title: Option<&'a str>,
    agent: Option<&'a str>,
    model: Option<&'a str>,
    state: Option<&'a str>,
    cwd: Option<&'a str>,
    context_pct: u32,
}

fn agent_tooltip(details: AgentTooltip<'_>) -> String {
    let mut lines = Vec::new();
    if let Some(agent) = details.agent {
        lines.push(agent.to_owned());
    }
    if let Some(title) = details.title {
        if details.agent != Some(title) {
            lines.push(title.to_owned());
        }
    }
    if let Some(model) = details.model {
        lines.push(format!("Model: {model}"));
    }
    if let Some(state) = details.state {
        lines.push(format!("State: {state}"));
    }
    if details.context_pct > 0 {
        lines.push(format!("Context: {}%", details.context_pct));
    }
    if let Some(cwd) = details.cwd {
        lines.push(cwd.to_owned());
    }

    if lines.is_empty() {
        "Agent".to_owned()
    } else {
        lines.join("\n")
    }
}

fn first_non_empty<'a>(values: &[Option<&'a str>]) -> Option<&'a str> {
    values.iter().find_map(|value| {
        let value = value.as_ref()?.trim();
        (!value.is_empty()).then_some(value)
    })
}

async fn read_project_icon(agent_session: &str) -> Option<String> {
    let project = read_link_node_id(&node_relation_path(agent_session, "session-project"))
        .await
        .ok()?;
    for property in ["display-icon", "icon"] {
        let value = read_optional_text(&node_property_path(&project, property)).await;
        if value.as_deref().is_some_and(|value| !value.is_empty()) {
            return value;
        }
    }
    None
}

async fn count_subagent_sessions(agent_session: &str) -> u32 {
    locusfs_client::read_dir_names(node_relation_path(agent_session, "subagent-session"))
        .await
        .map(|entries| entries.len().min(u32::MAX as usize) as u32)
        .unwrap_or(0)
}

async fn read_context_pct(path: &Path) -> u32 {
    let Some(value) = read_optional_text(path).await else {
        return 0;
    };

    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0).round() as u32)
        .unwrap_or(0)
}

async fn read_bool(path: &Path) -> Result<bool, SourceError> {
    match read_text(path).await?.as_str() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        value => Err(SourceError::new(format!("invalid bool value: {value}"))),
    }
}

async fn read_optional_text(path: &Path) -> Option<String> {
    read_text(path).await.ok().filter(|value| !value.is_empty())
}

async fn read_text(path: &Path) -> Result<String, SourceError> {
    Ok(locusfs_client::read_to_string(path)
        .await
        .map_err(|error| SourceError::new(format!("failed to read {}: {error}", path.display())))?
        .trim()
        .to_owned())
}

async fn read_link_node_id(path: &Path) -> Result<String, SourceError> {
    let path = locusfs_client::read_link(path).await.map_err(|error| {
        SourceError::new(format!("failed to resolve {}: {error}", path.display()))
    })?;

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

fn icon_for_app_id(app_id: &str) -> String {
    static CACHE: OnceLock<Mutex<DesktopIconCache>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(DesktopIconCache::load()));
    cache
        .lock()
        .expect("desktop icon cache lock is poisoned")
        .icon_for_app_id(app_id)
        .unwrap_or_default()
}

#[derive(Default)]
struct DesktopIconCache {
    entries: Vec<DesktopIconEntry>,
}

impl DesktopIconCache {
    fn load() -> Self {
        let mut entries = Vec::new();
        for directory in application_dirs() {
            collect_desktop_entries(&directory, &mut entries);
        }
        Self { entries }
    }

    fn icon_for_app_id(&mut self, app_id: &str) -> Option<String> {
        let normalized = normalize_app_id(app_id);
        if normalized.is_empty() {
            return None;
        }

        self.entries
            .iter()
            .find(|entry| entry.matches(&normalized))
            .and_then(|entry| entry.icon.clone())
    }
}

struct DesktopIconEntry {
    desktop_id: String,
    startup_wm_class: Option<String>,
    icon: Option<String>,
}

impl DesktopIconEntry {
    fn matches(&self, app_id: &str) -> bool {
        normalize_app_id(&self.desktop_id).contains(app_id)
            || self
                .startup_wm_class
                .as_ref()
                .is_some_and(|value| normalize_app_id(value).contains(app_id))
    }
}

fn collect_desktop_entries(directory: &Path, entries: &mut Vec<DesktopIconEntry>) {
    let Ok(files) = fs::read_dir(directory) else {
        return;
    };

    for file in files.flatten() {
        let path = file.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("desktop") {
            continue;
        }

        if let Some(entry) = read_desktop_entry(&path) {
            entries.push(entry);
        }
    }
}

fn read_desktop_entry(path: &Path) -> Option<DesktopIconEntry> {
    let desktop_id = path.file_name()?.to_str()?.to_owned();
    let content = fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut startup_wm_class = None;
    let mut icon = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }

        if let Some(value) = line.strip_prefix("StartupWMClass=") {
            startup_wm_class = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("Icon=") {
            icon = Some(value.to_owned());
        }

        if startup_wm_class.is_some() && icon.is_some() {
            break;
        }
    }

    Some(DesktopIconEntry {
        desktop_id,
        startup_wm_class,
        icon,
    })
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|value| {
            std::env::split_paths(&value)
                .map(|path| path.join("applications"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share/applications"),
                PathBuf::from("/usr/share/applications"),
            ]
        });
    dirs.extend(data_dirs);
    dirs
}

fn normalize_app_id(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(".desktop")
        .to_ascii_lowercase()
}

fn window_kind(title: &str) -> WindowTileKind {
    let title = title.to_ascii_lowercase();
    if title.contains("nvim") || title.contains("neovim") {
        WindowTileKind::Neovim
    } else {
        WindowTileKind::Plain
    }
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
