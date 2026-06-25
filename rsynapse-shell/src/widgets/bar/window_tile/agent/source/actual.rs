use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use super::super::{Agent, State};
use crate::widgets::bar::WindowNode;

const AGENT_SESSIONS_PATH: &str = "dbus/agentdbus/object/sessions/codex";
const AGENT_ICON: &str = "cognition";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AgentSession {
    window_id: Option<u32>,
    details: AgentSessionDetails,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AgentSessionDetails {
    status: String,
    requires_attention: bool,
    context_pct: u32,
}

pub(super) fn agent_for_window(window: WindowNode) -> Observable<Option<Agent>> {
    combine_latest!(
        window.observe_prop_or::<u32>("id", u32::MAX),
        agent_sessions()
            => |(window_id, sessions)|
                match_agent_session(window_id, &sessions)
                .map(|session| Agent {
                    icon: AGENT_ICON.to_owned(),
                    attention: session.details.requires_attention,
                    state: agent_state(&session.details.status),
                    context_pct: session.details.context_pct,
                }),
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
        properties.observe_prop_or::<bool>("RequiresAttention", false),
        properties.observe_prop_or::<String>("ContextPct", String::new())
            => move |(window_id, status, requires_attention, context_pct)| {
                AgentSession {
                    window_id: parse_window_id(&window_id),
                    details: AgentSessionDetails {
                        status,
                        requires_attention,
                        context_pct: parse_context_pct(&context_pct),
                    },
                }
            }
    )
    .distinct_until_changed()
    .box_it()
}

fn match_agent_session(window_id: u32, sessions: &[AgentSession]) -> Option<&AgentSession> {
    sessions
        .iter()
        .find(|session| session.window_id == Some(window_id))
}

fn agent_state(status: &str) -> State {
    let status = status.to_ascii_lowercase();
    if status.contains("compact") {
        State::Compacting
    } else if status.contains("tool") {
        State::ToolUse
    } else if status.contains("thinking")
        || status.contains("working")
        || status.contains("running")
        || status.contains("busy")
    {
        State::Thinking
    } else {
        State::None
    }
}

fn parse_window_id(window_id: &str) -> Option<u32> {
    let window_id = window_id.trim();
    if window_id.is_empty() {
        return None;
    }

    window_id.parse().ok()
}

fn parse_context_pct(context_pct: &str) -> u32 {
    let context_pct = context_pct.trim();
    if context_pct.is_empty() {
        return 0;
    }

    context_pct
        .parse::<u32>()
        .or_else(|_| context_pct.parse::<f64>().map(|value| value.round() as u32))
        .unwrap_or(0)
        .min(100)
}
