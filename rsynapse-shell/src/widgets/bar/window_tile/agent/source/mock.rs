use std::time::Duration;

use shell_core::source::{
    self, Observable,
    rx::{Observable as _, ObservableFactory as _, Shared},
};
use shell_rx_macros::combine_latest;

use super::super::{Agent, State};
use crate::widgets::bar::WindowNode;

const ENABLE_MOCK_AGENT_MATCHER: bool = false;
const MOCK_AGENT_UPDATE_INTERVAL: Duration = Duration::from_millis(900);
const AGENT_ICON: &str = "cognition";

#[derive(Clone, Debug, Eq, PartialEq)]
struct MockAgent {
    icon: String,
    attention: bool,
    state: State,
    context_pct: u32,
}

pub(super) fn agent_for_window(window: WindowNode) -> Observable<Option<Agent>> {
    if !ENABLE_MOCK_AGENT_MATCHER {
        return source::once(None);
    }

    let title = window.observe_prop_or::<String>("title", String::new());
    let app_id = window.observe_prop_or::<String>("app-id", String::new());
    let agent = combine_latest!(
        title,
        app_id
            => |(title, app_id)| mock_agent(&title, &app_id),
    )
    .distinct_until_changed();
    let tick = mock_agent_ticks();

    combine_latest!(
        agent,
        tick
            => |(agent, tick)| agent.as_ref().map(|agent| agent_at_tick(agent, tick)),
    )
    .distinct_until_changed()
    .box_it()
}

fn mock_agent(title: &str, app_id: &str) -> Option<MockAgent> {
    let title = title.to_ascii_lowercase();
    let app_id = app_id.to_ascii_lowercase();

    if !is_mock_agent_window(&title, &app_id) {
        return None;
    }

    Some(MockAgent {
        icon: AGENT_ICON.to_owned(),
        attention: title.contains("approval") || title.contains("review"),
        state: mock_state(&title),
        context_pct: mock_context_pct(&title),
    })
}

fn mock_agent_ticks() -> Observable<u64> {
    Shared::<()>::interval(MOCK_AGENT_UPDATE_INTERVAL)
        .start_with(vec![0])
        .map(|tick| tick as u64)
        .map_err(|error| error.to_string())
        .distinct_until_changed()
        .box_it()
}

fn agent_at_tick(agent: &MockAgent, tick: u64) -> Agent {
    Agent {
        icon: agent.icon.clone(),
        attention: agent.attention || matches!(tick % 8, 5 | 6),
        state: mock_state_at_tick(agent.state, tick),
        context_pct: mock_context_pct_at_tick(agent.context_pct, tick),
    }
}

fn mock_state_at_tick(base: State, tick: u64) -> State {
    match tick % 6 {
        0 => base,
        1 | 2 => State::Thinking,
        3 => State::ToolUse,
        4 => State::Compacting,
        _ => State::None,
    }
}

fn mock_context_pct_at_tick(base: u32, tick: u64) -> u32 {
    let offset = ((tick % 9) as u32) * 7;
    (base + offset).min(100)
}

fn is_mock_agent_window(title: &str, app_id: &str) -> bool {
    title.contains("codex")
        || title.contains("agent")
        || title.contains("locus-shell")
        || title.contains("cargo run")
        || app_id.contains("codex")
}

fn mock_state(title: &str) -> State {
    if title.contains("compact") {
        State::Compacting
    } else if title.contains("tool") || title.contains("cargo run") {
        State::ToolUse
    } else if title.contains("thinking") || title.contains("debug") {
        State::Thinking
    } else {
        State::None
    }
}

fn mock_context_pct(title: &str) -> u32 {
    if title.contains("compact") {
        96
    } else if title.contains("cargo run") {
        64
    } else {
        37
    }
}
