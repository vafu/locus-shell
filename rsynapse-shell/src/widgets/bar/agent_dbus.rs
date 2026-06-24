use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};

const AGENT_DBUS_OBJECTS_PATH: &str = "dbus-service/agentdbus/object/sessions/codex";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AgentSession {
    pub(super) path: LocusPath,
    pub(super) session_id: Option<String>,
    pub(super) agent: Option<String>,
    pub(super) nickname: Option<String>,
    pub(super) role: Option<String>,
    pub(super) model: Option<String>,
    pub(super) state: Option<String>,
    pub(super) cwd: Option<String>,
    pub(super) raw_title: Option<String>,
    pub(super) app_instance_id: Option<String>,
    pub(super) window_id: Option<u32>,
    pub(super) parent_session_id: Option<String>,
    pub(super) is_subagent: bool,
    pub(super) requires_attention: bool,
    pub(super) task_complete: bool,
    pub(super) context_pct: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ProjectPath {
    pub(super) path: LocusPath,
    pub(super) filesystem_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AgentSessionField {
    SessionId(Option<String>),
    Agent(Option<String>),
    Nickname(Option<String>),
    Role(Option<String>),
    Model(Option<String>),
    State(Option<String>),
    Cwd(Option<String>),
    RawTitle(Option<String>),
    AppInstanceId(Option<String>),
    WindowId(Option<u32>),
    ParentSessionId(Option<String>),
    IsSubagent(bool),
    RequiresAttention(bool),
    TaskComplete(bool),
    ContextPct(u32),
}

pub(super) fn agent_sessions() -> Observable<Vec<AgentSession>> {
    source::root()
        .child(AGENT_DBUS_OBJECTS_PATH)
        .as_children()
        .switch_map(|objects| {
            source::combine_latest(objects.into_iter().map(agent_session).collect())
        })
        .map(|sessions| {
            sessions
                .into_iter()
                .filter(|session| session.session_id.is_some())
                .collect::<Vec<_>>()
        })
        .distinct_until_changed()
        .box_it()
}

pub(super) fn projects() -> Observable<Vec<ProjectPath>> {
    source::root()
        .child("project")
        .as_children()
        .switch_map(|projects| {
            source::combine_latest(projects.into_iter().map(project_path).collect())
        })
        .map(|projects| {
            projects
                .into_iter()
                .filter(|project| !project.filesystem_path.is_empty())
                .collect::<Vec<_>>()
        })
        .distinct_until_changed()
        .box_it()
}

pub(super) fn project_for_cwd(cwd: Option<&str>, projects: &[ProjectPath]) -> Option<LocusPath> {
    let cwd = cwd?.trim();
    if cwd.is_empty() {
        return None;
    }

    projects
        .iter()
        .filter(|project| path_contains(&project.filesystem_path, cwd))
        .max_by_key(|project| project.filesystem_path.len())
        .map(|project| project.path.clone())
}

fn agent_session(path: LocusPath) -> Observable<AgentSession> {
    let properties = path.child("@properties");

    source::combine_latest(vec![
        properties
            .observe_prop::<String>("SessionId")
            .map(non_empty)
            .map(AgentSessionField::SessionId)
            .box_it(),
        properties
            .observe_prop::<String>("AgentName")
            .map(non_empty)
            .map(AgentSessionField::Agent)
            .box_it(),
        properties
            .observe_prop::<String>("AgentNickname")
            .map(non_empty)
            .map(AgentSessionField::Nickname)
            .box_it(),
        properties
            .observe_prop::<String>("AgentRole")
            .map(non_empty)
            .map(AgentSessionField::Role)
            .box_it(),
        properties
            .observe_prop::<String>("ModelName")
            .map(non_empty)
            .map(AgentSessionField::Model)
            .box_it(),
        properties
            .observe_prop::<String>("State")
            .map(non_empty)
            .map(AgentSessionField::State)
            .box_it(),
        properties
            .observe_prop::<String>("Cwd")
            .map(non_empty)
            .map(AgentSessionField::Cwd)
            .box_it(),
        properties
            .observe_prop::<String>("SessionTitle")
            .map(non_empty)
            .map(AgentSessionField::RawTitle)
            .box_it(),
        properties
            .observe_prop::<String>("AppInstanceId")
            .map(non_empty)
            .map(AgentSessionField::AppInstanceId)
            .box_it(),
        properties
            .observe_prop::<String>("WindowId")
            .map(parse_window_id)
            .map(AgentSessionField::WindowId)
            .box_it(),
        properties
            .observe_prop::<String>("ParentSessionId")
            .map(non_empty)
            .map(AgentSessionField::ParentSessionId)
            .box_it(),
        properties
            .observe_prop_or::<bool>("IsSubagent", false)
            .map(AgentSessionField::IsSubagent)
            .box_it(),
        properties
            .observe_prop_or::<bool>("RequiresAttention", false)
            .map(AgentSessionField::RequiresAttention)
            .box_it(),
        properties
            .observe_prop_or::<bool>("TaskComplete", false)
            .map(AgentSessionField::TaskComplete)
            .box_it(),
        properties
            .observe_prop_or::<f64>("ContextPct", 0.0)
            .map(percent)
            .map(AgentSessionField::ContextPct)
            .box_it(),
    ])
    .map(move |fields| agent_session_from_fields(path.clone(), fields))
    .distinct_until_changed()
    .box_it()
}

fn agent_session_from_fields(path: LocusPath, fields: Vec<AgentSessionField>) -> AgentSession {
    let mut session = AgentSession {
        path,
        session_id: None,
        agent: None,
        nickname: None,
        role: None,
        model: None,
        state: None,
        cwd: None,
        raw_title: None,
        app_instance_id: None,
        window_id: None,
        parent_session_id: None,
        is_subagent: false,
        requires_attention: false,
        task_complete: false,
        context_pct: 0,
    };

    for field in fields {
        match field {
            AgentSessionField::SessionId(value) => session.session_id = value,
            AgentSessionField::Agent(value) => session.agent = value,
            AgentSessionField::Nickname(value) => session.nickname = value,
            AgentSessionField::Role(value) => session.role = value,
            AgentSessionField::Model(value) => session.model = value,
            AgentSessionField::State(value) => session.state = value,
            AgentSessionField::Cwd(value) => session.cwd = value,
            AgentSessionField::RawTitle(value) => session.raw_title = value,
            AgentSessionField::AppInstanceId(value) => session.app_instance_id = value,
            AgentSessionField::WindowId(value) => session.window_id = value,
            AgentSessionField::ParentSessionId(value) => session.parent_session_id = value,
            AgentSessionField::IsSubagent(value) => session.is_subagent = value,
            AgentSessionField::RequiresAttention(value) => session.requires_attention = value,
            AgentSessionField::TaskComplete(value) => session.task_complete = value,
            AgentSessionField::ContextPct(value) => session.context_pct = value,
        }
    }

    session
}

fn project_path(project: LocusPath) -> Observable<ProjectPath> {
    project
        .observe_prop_or::<String>("path", String::new())
        .map(move |filesystem_path| ProjectPath {
            path: project.clone(),
            filesystem_path,
        })
        .distinct_until_changed()
        .box_it()
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}

fn parse_window_id(value: Option<String>) -> Option<u32> {
    value?.trim().parse().ok()
}

fn percent(value: f64) -> u32 {
    Some(value)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0).round() as u32)
        .unwrap_or(0)
}

fn path_contains(project: &str, cwd: &str) -> bool {
    cwd == project
        || cwd
            .strip_prefix(project)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
