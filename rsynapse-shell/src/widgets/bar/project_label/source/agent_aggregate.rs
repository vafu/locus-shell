use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use super::super::super::agent_dbus::{self, AgentSession, ProjectPath};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct AgentAggregate {
    pub(super) has_attention: bool,
    pub(super) all_idle: bool,
    pub(super) has_working: bool,
    pub(super) has_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSessionModel {
    project: Option<LocusPath>,
    state: Option<String>,
    requires_attention: bool,
    task_complete: bool,
    window_id: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WindowModel {
    id: Option<u32>,
    workspace: Option<LocusPath>,
}

pub(super) fn project_agent_aggregate_source(
    workspace: LocusPath,
    project: Option<LocusPath>,
) -> Observable<AgentAggregate> {
    match project {
        Some(project) => project_agent_aggregate(workspace, project),
        None => source::once(AgentAggregate::default()),
    }
}

fn project_agent_aggregate(workspace: LocusPath, project: LocusPath) -> Observable<AgentAggregate> {
    let project_paths = agent_dbus::projects();
    let sessions =
        agent_dbus::agent_sessions().combine_latest(project_paths, |sessions, projects| {
            sessions
                .into_iter()
                .map(|session| agent_session_model(session, &projects))
                .collect::<Vec<_>>()
        });
    let windows = source::root()
        .child("window")
        .as_children()
        .switch_map(|windows| {
            source::combine_latest_vec(windows.into_iter().map(window_model).collect())
        });

    combine_latest!(
        sessions,
        windows
            => move |(sessions, windows)| {
                aggregate_agents(&workspace, &project, sessions, windows)
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn aggregate_agents(
    workspace: &LocusPath,
    project: &LocusPath,
    sessions: Vec<AgentSessionModel>,
    windows: Vec<WindowModel>,
) -> AgentAggregate {
    let workspace_id = workspace.node_id().ok();
    let project_id = project.node_id().ok();
    let agents = sessions
        .into_iter()
        .filter(|session| {
            if let Some(session_workspace) = windows
                .iter()
                .find(|window| window.id.is_some() && window.id == session.window_id)
                .and_then(|window| window.workspace.as_ref())
            {
                return session_workspace.node_id().ok() == workspace_id;
            }

            session
                .project
                .as_ref()
                .and_then(|project| project.node_id().ok())
                == project_id
        })
        .collect::<Vec<_>>();
    let count = agents.len();

    AgentAggregate {
        has_attention: agents.iter().any(|agent| agent.requires_attention),
        all_idle: count > 0
            && agents
                .iter()
                .all(|agent| agent.state.as_deref() == Some("idle")),
        has_working: agents.iter().any(|agent| {
            matches!(
                agent.state.as_deref(),
                Some("thinking" | "tool-use" | "compacting")
            )
        }),
        has_complete: agents.iter().any(|agent| agent.task_complete),
    }
}

fn agent_session_model(session: AgentSession, projects: &[ProjectPath]) -> AgentSessionModel {
    AgentSessionModel {
        project: agent_dbus::project_for_cwd(session.cwd.as_deref(), projects),
        state: session.state,
        requires_attention: session.requires_attention,
        task_complete: session.task_complete,
        window_id: session.window_id,
    }
}

fn window_model(window: LocusPath) -> Observable<WindowModel> {
    combine_latest!(
        window.observe_prop::<u32>("id"),
        window.observe_rel("workspace")
            => |(id, workspace)| WindowModel {
                id,
                workspace,
            },
    )
    .distinct_until_changed()
    .box_it()
}
