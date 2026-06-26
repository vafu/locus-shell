use shell_core::source::{Observable, rx::Observable as _};
use shell_rx_macros::combine_latest;

use super::super::{Agent, State};
use crate::widgets::bar::{
    WindowNode,
    agent_sessions::{AgentSessionSnapshot, agent_sessions},
};
const AGENT_ICON: &str = "cognition";

pub(super) fn agent_for_window(window: WindowNode) -> Observable<Option<Agent>> {
    combine_latest!(
        window.observe_prop_or::<u32>("id", u32::MAX),
        agent_sessions()
            => |(window_id, sessions)|
                match_agent_session(window_id, &sessions)
                .map(|session| Agent {
                    icon: AGENT_ICON.to_owned(),
                    attention: session.requires_attention,
                    state: agent_state(&session.status),
                    context_pct: session.context_pct,
                }),
    )
    .distinct_until_changed()
    .box_it()
}

fn match_agent_session(
    window_id: u32,
    sessions: &[AgentSessionSnapshot],
) -> Option<&AgentSessionSnapshot> {
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
