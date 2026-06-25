use std::collections::BTreeSet;

use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

const AGENT_SESSIONS_PATH: &str = "dbus/agentdbus/object/sessions/codex";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::widgets::bar::project_label) struct WorkspaceAgentState {
    pub(in crate::widgets::bar::project_label) has_attention: bool,
    pub(in crate::widgets::bar::project_label) has_working: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceWindow {
    workspace_id: Option<u32>,
    window_id: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSession {
    window_id: Option<u32>,
    status: String,
    requires_attention: bool,
}

pub(super) fn workspace_agent_state(workspace: LocusPath) -> Observable<WorkspaceAgentState> {
    combine_latest!(
        workspace.observe_prop_or::<u32>("id", u32::MAX),
        workspace_windows(),
        agent_sessions()
            => |(workspace_id, windows, sessions)| {
                let workspace_window_ids = windows
                    .iter()
                    .filter(|window| window.workspace_id == Some(workspace_id))
                    .map(|window| window.window_id)
                    .collect::<BTreeSet<_>>();

                sessions
                    .iter()
                    .filter(|session| {
                        session
                            .window_id
                            .is_some_and(|window_id| workspace_window_ids.contains(&window_id))
                    })
                    .fold(WorkspaceAgentState::default(), |mut state, session| {
                        state.has_attention |= session.requires_attention;
                        state.has_working |= is_working_status(&session.status);
                        state
                    })
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn workspace_windows() -> Observable<Vec<WorkspaceWindow>> {
    source::root()
        .child("window")
        .as_children()
        .switch_map(|windows| {
            let window_sources: Vec<Observable<WorkspaceWindow>> =
                windows.into_iter().map(workspace_window).collect();
            source::combine_latest_vec::<WorkspaceWindow>(window_sources)
        })
        // Keeps RxRust's boxed switch_map observer inference stable.
        .map(|windows| windows)
        .distinct_until_changed()
        .box_it()
}

fn workspace_window(window: LocusPath) -> Observable<WorkspaceWindow> {
    combine_latest!(
        window.observe_prop_or::<u32>("workspace-id", u32::MAX),
        window.observe_prop_or::<u32>("id", u32::MAX)
            => |(workspace_id, window_id)| WorkspaceWindow {
                workspace_id: Some(workspace_id),
                window_id,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn agent_sessions() -> Observable<Vec<AgentSession>> {
    source::root()
        .child(AGENT_SESSIONS_PATH)
        .as_children()
        .switch_map(|sessions| {
            let session_sources: Vec<Observable<AgentSession>> =
                sessions.into_iter().map(agent_session).collect();
            source::combine_latest_vec::<AgentSession>(session_sources)
        })
        // Keeps RxRust's boxed switch_map observer inference stable.
        .map(|sessions| sessions)
        .distinct_until_changed()
        .box_it()
}

fn agent_session(session: LocusPath) -> Observable<AgentSession> {
    let properties = session.child("@properties");
    combine_latest!(
        properties.observe_prop_or::<String>("WindowId", String::new()),
        properties.observe_prop_or::<String>("State", String::new()),
        properties.observe_prop_or::<bool>("RequiresAttention", false)
            => |(window_id, status, requires_attention)| AgentSession {
                window_id: parse_window_id(&window_id),
                status,
                requires_attention,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn parse_window_id(window_id: &str) -> Option<u32> {
    let window_id = window_id.trim();
    if window_id.is_empty() {
        return None;
    }

    window_id.parse().ok()
}

fn is_working_status(status: &str) -> bool {
    let status = status.to_ascii_lowercase();
    status.contains("compact")
        || status.contains("tool")
        || status.contains("thinking")
        || status.contains("working")
        || status.contains("running")
        || status.contains("busy")
}
