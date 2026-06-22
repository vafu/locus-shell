use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, RelationState, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use super::source::{AgentVisualState, WindowBase, WindowTileKind, WindowTileVm, non_empty};

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

pub(super) fn agent_tile_source(
    base: WindowBase,
    app_instance: LocusPath,
    agent_session: LocusPath,
) -> Observable<WindowTileVm> {
    let app = agent_app_props_source(app_instance);
    let session = agent_session_props_source(agent_session.clone());
    let project_icon = project_icon_source(agent_session.clone());
    let substatus_count = substatus_count_source(agent_session);

    combine_latest!(
        app,
        session,
        project_icon,
        substatus_count
            => move |(app, session, project_icon, substatus_count)| {
                agent_window_tile(
                    &base,
                    AgentTileDetails {
                        app,
                        session,
                        project_icon,
                    },
                    substatus_count,
                )
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn agent_app_props_source(app_instance: LocusPath) -> Observable<AgentAppProps> {
    combine_latest!(
        app_instance.observe_prop::<String>("icon").map(non_empty),
        app_instance.observe_prop::<String>("name").map(non_empty)
            => |(icon, name)| AgentAppProps { icon, name },
    )
    .distinct_until_changed()
    .box_it()
}

fn agent_session_props_source(agent_session: LocusPath) -> Observable<AgentSessionProps> {
    combine_latest!(
        agent_session.observe_prop::<String>("agent").map(non_empty),
        agent_session
            .observe_prop::<String>("agent_nickname")
            .map(non_empty),
        agent_session.observe_prop::<String>("model").map(non_empty),
        agent_session.observe_prop::<String>("state").map(non_empty),
        agent_session.observe_prop::<String>("cwd").map(non_empty),
        agent_session
            .observe_prop::<String>("raw_title")
            .map(non_empty),
        agent_session
            .observe_prop_or::<bool>("requires_attention", false),
        agent_session
            .observe_prop_or::<f64>("context_pct", 0.0)
            .map(|value| {
                Some(value)
                    .filter(|value| value.is_finite())
                    .map(|value| value.clamp(0.0, 100.0).round() as u32)
                    .unwrap_or(0)
            })
            => |(
                agent,
                nickname,
                model,
                state,
                cwd,
                raw_title,
                requires_attention,
                context_pct,
            )| AgentSessionProps {
                agent,
                nickname,
                model,
                state,
                cwd,
                raw_title,
                requires_attention,
                context_pct,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn project_icon_source(agent_session: LocusPath) -> Observable<Option<String>> {
    agent_session
        .observe_rel("session-project")
        .map(|project| match project {
            Some(project) => RelationState::Set(project),
            None => RelationState::Unset,
        })
        .switch_map(|project| match project {
            RelationState::Unset => source::once(ProjectIcon(None)),
            RelationState::Set(project) => combine_latest!(
                project.observe_prop::<String>("display-icon").map(non_empty),
                project.observe_prop::<String>("icon").map(non_empty)
                    => |(display_icon, icon)| ProjectIcon(display_icon.or(icon)),
            )
            .distinct_until_changed()
            .box_it(),
        })
        .map(|icon| icon.0)
        .distinct_until_changed()
        .box_it()
}

fn substatus_count_source(agent_session: LocusPath) -> Observable<u32> {
    agent_session
        .rel("subagent-session")
        .as_children()
        .map(|entries| entries.len().min(u32::MAX as usize) as u32)
        .box_it()
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
        agent_state: agent_visual_state(details.session.state.as_deref()),
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
        urgent: base.urgent,
        attention: details.session.requires_attention,
        context_pct: details.session.context_pct,
        substatus_count,
    }
}

fn agent_visual_state(state: Option<&str>) -> AgentVisualState {
    match state {
        Some("thinking") => AgentVisualState::Thinking,
        Some("tool-use") => AgentVisualState::ToolUse,
        Some("compacting") => AgentVisualState::Compacting,
        _ => AgentVisualState::None,
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
    if let Some(title) = details.title
        && details.agent != Some(title)
    {
        lines.push(title.to_owned());
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
