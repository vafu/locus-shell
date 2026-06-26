use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use crate::locusfs_paths::AGENTDBUS;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) struct AgentSessionSnapshot {
    pub(in crate::widgets::bar) window_id: Option<u32>,
    pub(in crate::widgets::bar) status: String,
    pub(in crate::widgets::bar) requires_attention: bool,
    pub(in crate::widgets::bar) context_pct: u32,
}

pub(in crate::widgets::bar) fn agent_sessions() -> Observable<Vec<AgentSessionSnapshot>> {
    source::shared_by_key("rsynapse.agentdbus.sessions", "codex", || {
        AGENTDBUS
            .object("sessions/codex")
            .as_children()
            .switch_map(|sessions| {
                source::combine_latest_vec::<AgentSessionSnapshot>(
                    sessions.into_iter().map(agent_session).collect(),
                )
            })
            .map(|sessions| sessions)
            .distinct_until_changed()
            .box_it()
    })
}

fn agent_session(session: LocusPath) -> Observable<AgentSessionSnapshot> {
    combine_latest!(
        session.observe_prop_or::<String>("WindowId", String::new()),
        session.observe_prop_or::<String>("State", String::new()),
        session.observe_prop_or::<bool>("RequiresAttention", false),
        session.observe_prop_or::<String>("ContextPct", String::new())
            => move |(window_id, status, requires_attention, context_pct)| {
                AgentSessionSnapshot {
                    window_id: parse_window_id(&window_id),
                    status,
                    requires_attention,
                    context_pct: parse_context_pct(&context_pct),
                }
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
