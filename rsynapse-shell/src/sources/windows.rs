use std::{
    convert::Infallible,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use shell_core::{
    locus_path::{LocusPath, node_id_from_path},
    source::{self, Observable, rx::Observable as _},
};

use crate::sources::WindowNode;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowTileKind {
    #[default]
    Plain,
    Agent,
    Neovim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowTileVm {
    pub kind: WindowTileKind,
    pub icon: String,
    pub tooltip: String,
    pub active: bool,
    pub urgent: bool,
    pub context_pct: u32,
    pub substatus_count: u32,
}

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";

pub(crate) fn window_tile_vm(window: WindowNode) -> Observable<Option<WindowTileVm>> {
    window_base_source(window)
        .switch_map(agent_session_source)
        .switch_map(tile_vm_source)
        .map(Some)
        .distinct_until_changed()
        .box_it()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WindowBase {
    title: String,
    app_id: String,
    icon: String,
    active: bool,
    urgent: bool,
    app_instance: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentResolvedWindow {
    base: WindowBase,
    agent_session: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentAppProps {
    icon: Option<String>,
    name: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AgentSessionProps {
    agent: Option<String>,
    nickname: Option<String>,
    model: Option<String>,
    state: Option<String>,
    cwd: Option<String>,
    raw_title: Option<String>,
    requires_attention: bool,
    context_pct: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentTileDetails {
    app: AgentAppProps,
    session: AgentSessionProps,
    project_icon: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectIcon(Option<String>);

fn window_base_source(window: WindowNode) -> Observable<WindowBase> {
    let window = root().node(&window);
    let title = source::property::<String>(window.prop("title")).map(non_empty);
    let app_id = source::property::<String>(window.prop("app-id")).map(non_empty);
    let active =
        source::property::<bool>(window.prop("selected")).map(|value| value.unwrap_or(false));
    let urgent =
        source::property::<bool>(window.prop("urgent")).map(|value| value.unwrap_or(false));
    let app_instance = source::relation(window.rel("app-instance"))
        .map(|target| target.and_then(|target| node_id_from_path(target).ok()));

    title
        .combine_latest(app_id, |title, app_id| {
            (title.unwrap_or_default(), app_id.unwrap_or_default())
        })
        .combine_latest(active, |(title, app_id), active| (title, app_id, active))
        .combine_latest(urgent, |(title, app_id, active), urgent| {
            (title, app_id, active, urgent)
        })
        .combine_latest(
            app_instance,
            |(title, app_id, active, urgent), app_instance| {
                let icon = icon_for_app_id(&app_id);
                WindowBase {
                    title,
                    app_id,
                    icon,
                    active,
                    urgent,
                    app_instance,
                }
            },
        )
        .distinct_until_changed()
        .box_it()
}

fn agent_session_source(base: WindowBase) -> Observable<AgentResolvedWindow> {
    let Some(app_instance) = base.app_instance.clone() else {
        return source::once(AgentResolvedWindow {
            base,
            agent_session: None,
        })
        .map_err(|error: Infallible| match error {})
        .box_it();
    };

    source::relation(root().node(&app_instance).rel("agent-session"))
        .map(|target| target.and_then(|target| node_id_from_path(&target).ok()))
        .map(move |agent_session| AgentResolvedWindow {
            base: base.clone(),
            agent_session,
        })
        .distinct_until_changed()
        .box_it()
}

fn tile_vm_source(window: AgentResolvedWindow) -> Observable<WindowTileVm> {
    let Some(app_instance) = window.base.app_instance.clone() else {
        return source::once(plain_window_tile(&window.base))
            .map_err(|error: Infallible| match error {})
            .box_it();
    };
    let Some(agent_session) = window.agent_session.clone() else {
        return source::once(plain_window_tile(&window.base))
            .map_err(|error: Infallible| match error {})
            .box_it();
    };

    agent_tile_source(window.base, app_instance, agent_session)
}

fn agent_tile_source(
    base: WindowBase,
    app_instance: String,
    agent_session: String,
) -> Observable<WindowTileVm> {
    let app = agent_app_props_source(app_instance);
    let session = agent_session_props_source(agent_session.clone());
    let project_icon = project_icon_source(agent_session.clone());
    let substatus_count = substatus_count_source(agent_session);

    app.combine_latest(session, |app, session| (app, session))
        .combine_latest(project_icon, |(app, session), project_icon| {
            AgentTileDetails {
                app,
                session,
                project_icon,
            }
        })
        .combine_latest(substatus_count, move |details, substatus_count| {
            agent_window_tile(&base, details, substatus_count)
        })
        .distinct_until_changed()
        .box_it()
}

fn agent_app_props_source(app_instance: String) -> Observable<AgentAppProps> {
    let app_instance = root().node(&app_instance);
    source::property::<String>(app_instance.prop("icon"))
        .map(non_empty)
        .combine_latest(
            source::property::<String>(app_instance.prop("name")).map(non_empty),
            |icon, name| AgentAppProps { icon, name },
        )
        .distinct_until_changed()
        .box_it()
}

fn agent_session_props_source(agent_session: String) -> Observable<AgentSessionProps> {
    let agent_session = root().node(&agent_session);
    source::property::<String>(agent_session.prop("agent"))
        .map(non_empty)
        .combine_latest(
            source::property::<String>(agent_session.prop("agent_nickname")).map(non_empty),
            |agent, nickname| AgentSessionProps {
                agent,
                nickname,
                ..AgentSessionProps::default()
            },
        )
        .combine_latest(
            source::property::<String>(agent_session.prop("model")).map(non_empty),
            |mut props, model| {
                props.model = model;
                props
            },
        )
        .combine_latest(
            source::property::<String>(agent_session.prop("state")).map(non_empty),
            |mut props, state| {
                props.state = state;
                props
            },
        )
        .combine_latest(
            source::property::<String>(agent_session.prop("cwd")).map(non_empty),
            |mut props, cwd| {
                props.cwd = cwd;
                props
            },
        )
        .combine_latest(
            source::property::<String>(agent_session.prop("raw_title")).map(non_empty),
            |mut props, raw_title| {
                props.raw_title = raw_title;
                props
            },
        )
        .combine_latest(
            source::property::<bool>(agent_session.prop("requires_attention"))
                .map(|value| value.unwrap_or(false)),
            |mut props, requires_attention| {
                props.requires_attention = requires_attention;
                props
            },
        )
        .combine_latest(
            source::property::<f64>(agent_session.prop("context_pct")).map(|value| {
                value
                    .filter(|value| value.is_finite())
                    .map(|value| value.clamp(0.0, 100.0).round() as u32)
                    .unwrap_or(0)
            }),
            |mut props, context_pct| {
                props.context_pct = context_pct;
                props
            },
        )
        .distinct_until_changed()
        .box_it()
}

fn project_icon_source(agent_session: String) -> Observable<Option<String>> {
    source::relation(root().node(&agent_session).rel("session-project"))
        .map(|target| target.and_then(|target| node_id_from_path(&target).ok()))
        .switch_map(project_icon_for_relation)
        .map(|icon| icon.0)
        .distinct_until_changed()
        .box_it()
}

fn project_icon_for_relation(project: Option<String>) -> Observable<ProjectIcon> {
    match project {
        Some(project) => project_icon_props_source(project),
        None => source::once(ProjectIcon(None))
            .map_err(|error: Infallible| match error {})
            .box_it(),
    }
}

fn project_icon_props_source(project: String) -> Observable<ProjectIcon> {
    let project = root().node(&project);
    source::property::<String>(project.prop("display-icon"))
        .map(non_empty)
        .combine_latest(
            source::property::<String>(project.prop("icon")).map(non_empty),
            |display_icon, icon| ProjectIcon(display_icon.or(icon)),
        )
        .distinct_until_changed()
        .box_it()
}

fn substatus_count_source(agent_session: String) -> Observable<u32> {
    source::children(root().node(&agent_session).rel("subagent-session"))
        .map(|entries| entries.len().min(u32::MAX as usize) as u32)
        .box_it()
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn plain_window_tile(base: &WindowBase) -> WindowTileVm {
    WindowTileVm {
        kind: window_kind(&base.title),
        icon: base.icon.clone(),
        tooltip: base.title.clone(),
        active: base.active,
        urgent: base.urgent,
        context_pct: 0,
        substatus_count: 0,
    }
}

fn agent_window_tile(
    base: &WindowBase,
    details: AgentTileDetails,
    substatus_count: u32,
) -> WindowTileVm {
    let icon = if let Some(icon) = details.project_icon {
        icon
    } else if let Some(icon) = details.app.icon {
        icon
    } else if base.icon.is_empty() {
        "smart_toy".to_owned()
    } else {
        base.icon.clone()
    };

    WindowTileVm {
        kind: WindowTileKind::Agent,
        icon,
        tooltip: agent_tooltip(AgentTooltip {
            title: first_non_empty(&[details.session.raw_title.as_deref(), Some(&base.title)]),
            agent: first_non_empty(&[
                details.session.nickname.as_deref(),
                details.session.agent.as_deref(),
                details.app.name.as_deref(),
            ]),
            model: details.session.model.as_deref(),
            state: details.session.state.as_deref(),
            cwd: details.session.cwd.as_deref(),
            context_pct: details.session.context_pct,
        }),
        active: base.active,
        urgent: base.urgent || details.session.requires_attention,
        context_pct: details.session.context_pct,
        substatus_count,
    }
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

fn root() -> LocusPath {
    LocusPath::from_env_or(ROOT_ENV, DEFAULT_ROOT)
}
