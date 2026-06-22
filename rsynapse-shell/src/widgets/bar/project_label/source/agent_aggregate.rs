use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use super::non_empty;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct AgentAggregate {
    pub(super) has_attention: bool,
    pub(super) all_idle: bool,
    pub(super) has_working: bool,
    pub(super) has_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSessionModel {
    path: LocusPath,
    project: Option<LocusPath>,
    state: Option<String>,
    requires_attention: bool,
    task_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AppInstanceModel {
    path: LocusPath,
    agent_session: Option<LocusPath>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WindowModel {
    app_instance: Option<LocusPath>,
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
    let sessions = source::root()
        .child("agent-session")
        .as_children()
        .switch_map(|sessions| {
            source::combine_latest_vec(sessions.into_iter().map(agent_session_model).collect())
        });
    let app_instances = source::root()
        .child("app-instance")
        .as_children()
        .switch_map(|app_instances| {
            source::combine_latest_vec(app_instances.into_iter().map(app_instance_model).collect())
        });
    let windows = source::root()
        .child("window")
        .as_children()
        .switch_map(|windows| {
            source::combine_latest_vec(windows.into_iter().map(window_model).collect())
        });

    combine_latest!(
        sessions,
        app_instances,
        windows
            => move |(sessions, app_instances, windows)| {
                aggregate_agents(&workspace, &project, sessions, app_instances, windows)
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn aggregate_agents(
    workspace: &LocusPath,
    project: &LocusPath,
    sessions: Vec<AgentSessionModel>,
    app_instances: Vec<AppInstanceModel>,
    windows: Vec<WindowModel>,
) -> AgentAggregate {
    let workspace_id = workspace.node_id().ok();
    let project_id = project.node_id().ok();
    let agents = sessions
        .into_iter()
        .filter(|session| {
            let session_id = session.path.node_id().ok();
            let session_window_workspaces = app_instances
                .iter()
                .filter(|app| {
                    app.agent_session
                        .as_ref()
                        .and_then(|path| path.node_id().ok())
                        == session_id
                })
                .flat_map(|app| {
                    let app_id = app.path.node_id().ok();
                    windows
                        .iter()
                        .filter(move |window| {
                            window
                                .app_instance
                                .as_ref()
                                .and_then(|path| path.node_id().ok())
                                == app_id
                        })
                        .filter_map(|window| window.workspace.as_ref())
                })
                .collect::<Vec<_>>();

            if !session_window_workspaces.is_empty() {
                return session_window_workspaces
                    .iter()
                    .any(|workspace| workspace.node_id().ok() == workspace_id);
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

fn agent_session_model(session: LocusPath) -> Observable<AgentSessionModel> {
    combine_latest!(
        session.observe_rel("session-project"),
        session.observe_prop::<String>("state").map(non_empty),
        session.observe_prop_or::<bool>("requires_attention", false),
        session.observe_prop_or::<bool>("task_complete", false)
            => move |(project, state, requires_attention, task_complete)| AgentSessionModel {
                path: session.clone(),
                project,
                state,
                requires_attention,
                task_complete,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn app_instance_model(app_instance: LocusPath) -> Observable<AppInstanceModel> {
    app_instance
        .observe_rel("agent-session")
        .map(move |agent_session| AppInstanceModel {
            path: app_instance.clone(),
            agent_session,
        })
        .distinct_until_changed()
        .box_it()
}

fn window_model(window: LocusPath) -> Observable<WindowModel> {
    combine_latest!(
        window.observe_rel("app-instance"),
        window.observe_rel("workspace")
            => |(app_instance, workspace)| WindowModel {
                app_instance,
                workspace,
            },
    )
    .distinct_until_changed()
    .box_it()
}
