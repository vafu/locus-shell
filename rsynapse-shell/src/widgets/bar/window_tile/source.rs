use shell_core::source::{self, Observable, rx::Observable as _};
use shell_rx_macros::combine_latest;

use crate::desktop_icon;

use super::super::WindowNode;

use super::agent;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::widgets::bar) enum WindowTileKind {
    #[default]
    Plain,
    Agent,
    Neovim,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::widgets::bar) enum AgentVisualState {
    #[default]
    None,
    Thinking,
    ToolUse,
    Compacting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) struct WindowTileVm {
    pub(super) kind: WindowTileKind,
    pub(super) agent_state: AgentVisualState,
    pub(super) icon: String,
    pub(super) tooltip: String,
    pub(super) active: bool,
    pub(super) urgent: bool,
    pub(super) attention: bool,
    pub(super) context_pct: u32,
    pub(super) substatus_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct WindowBase {
    pub(super) title: String,
    pub(super) icon: String,
    pub(super) active: bool,
    pub(super) urgent: bool,
    id: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentResolvedWindow {
    base: WindowBase,
    agent_session: Option<super::super::agent_dbus::AgentSession>,
}

pub(super) fn window_tile_vm(window: WindowNode) -> Observable<Option<WindowTileVm>> {
    window_base_source(window)
        .switch_map(agent_session_source)
        .switch_map(tile_vm_source)
        .map(Some)
        .distinct_until_changed()
        .box_it()
}

fn window_base_source(window: WindowNode) -> Observable<WindowBase> {
    let title = window.observe_prop::<String>("title").map(non_empty);
    let app_id = window.observe_prop::<String>("app-id").map(non_empty);
    let active = window.observe_prop_or::<bool>("selected", false);
    let urgent = window.observe_prop_or::<bool>("urgent", false);
    let id = window.observe_prop::<u32>("id");

    combine_latest!(
        title,
        app_id,
        active,
        urgent,
        id
            => |(title, app_id, active, urgent, id)| {
                let app_id = app_id.unwrap_or_default();
                WindowBase {
                    title: title.unwrap_or_default(),
                    icon: desktop_icon::icon_for_app_id(&app_id),
                    active,
                    urgent,
                    id,
                }
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn agent_session_source(base: WindowBase) -> Observable<AgentResolvedWindow> {
    let window_id = base.id;
    super::super::agent_dbus::agent_sessions()
        .map(move |sessions| AgentResolvedWindow {
            base: base.clone(),
            agent_session: sessions
                .into_iter()
                .find(|session| window_id.is_some() && session.window_id == window_id),
        })
        .distinct_until_changed()
        .box_it()
}

fn tile_vm_source(window: AgentResolvedWindow) -> Observable<WindowTileVm> {
    let Some(agent_session) = window.agent_session.clone() else {
        return source::once(plain_window_tile(&window.base));
    };

    agent::agent_tile_source(window.base, agent_session)
}

fn plain_window_tile(base: &WindowBase) -> WindowTileVm {
    WindowTileVm {
        kind: window_kind(&base.title),
        agent_state: AgentVisualState::None,
        icon: base.icon.clone(),
        tooltip: base.title.clone(),
        active: base.active,
        urgent: base.urgent,
        attention: false,
        context_pct: 0,
        substatus_count: 0,
    }
}

fn window_kind(title: &str) -> WindowTileKind {
    let title = title.to_ascii_lowercase();
    if title.contains("nvim") || title.contains("neovim") {
        WindowTileKind::Neovim
    } else {
        WindowTileKind::Plain
    }
}

pub(super) fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}
