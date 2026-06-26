use std::collections::BTreeMap;

use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use crate::locusfs_paths::DBUS_SESSION;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) struct AgentSessionSnapshot {
    pub(in crate::widgets::bar) session_id: String,
    pub(in crate::widgets::bar) window_id: Option<u32>,
    pub(in crate::widgets::bar) status: String,
    pub(in crate::widgets::bar) requires_attention: bool,
    pub(in crate::widgets::bar) context_pct: u32,
}

pub(in crate::widgets::bar) fn agent_sessions() -> Observable<Vec<AgentSessionSnapshot>> {
    source::shared_by_key("rsynapse.agentdbus.sessions", "codex", || {
        DBUS_SESSION
            .object("/io/github/AgentDBus/sessions/codex")
            .as_children()
            .map(agent_session_children)
            .switch_map(|sessions| {
                source::combine_latest_vec::<AgentSessionSnapshot>(
                    sessions.into_iter().map(agent_session).collect(),
                )
            })
            .map(latest_sessions_by_window)
            .distinct_until_changed()
            .box_it()
    })
}

fn agent_session_children(children: Vec<LocusPath>) -> Vec<LocusPath> {
    children
        .into_iter()
        .filter(is_agent_session_child)
        .collect()
}

fn is_agent_session_child(child: &LocusPath) -> bool {
    child
        .as_path()
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.ends_with(".call"))
}

fn agent_session(session: LocusPath) -> Observable<AgentSessionSnapshot> {
    let session_id = session_id_from_path(&session);
    combine_latest!(
        session.observe_prop_or::<String>("SessionId", session_id),
        session.observe_prop_or::<String>("WindowId", String::new()),
        session.observe_prop_or::<String>("State", String::new()),
        session.observe_prop_or::<bool>("RequiresAttention", false),
        session.observe_prop_or::<String>("ContextPct", String::new())
            => move |(session_id, window_id, status, requires_attention, context_pct)| {
                AgentSessionSnapshot {
                    session_id,
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

fn latest_sessions_by_window(sessions: Vec<AgentSessionSnapshot>) -> Vec<AgentSessionSnapshot> {
    let mut latest_by_window = BTreeMap::new();
    let mut unbound = Vec::new();

    for session in sessions {
        let Some(window_id) = session.window_id else {
            unbound.push(session);
            continue;
        };

        match latest_by_window.entry(window_id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(session);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if session_is_newer(&session, entry.get()) {
                    entry.insert(session);
                }
            }
        }
    }

    unbound.extend(latest_by_window.into_values());
    unbound
}

fn session_is_newer(candidate: &AgentSessionSnapshot, current: &AgentSessionSnapshot) -> bool {
    candidate.session_id > current.session_id
}

fn session_id_from_path(session: &LocusPath) -> String {
    session
        .as_path()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
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

#[cfg(test)]
mod tests {
    use shell_core::locus_path::LocusPath;

    use super::{AgentSessionSnapshot, agent_session_children, latest_sessions_by_window};

    fn session(session_id: &str, window_id: Option<u32>, status: &str) -> AgentSessionSnapshot {
        AgentSessionSnapshot {
            session_id: session_id.to_owned(),
            window_id,
            status: status.to_owned(),
            requires_attention: false,
            context_pct: 0,
        }
    }

    #[test]
    fn latest_sessions_by_window_prefers_newer_session_for_same_window() {
        assert_eq!(
            latest_sessions_by_window(vec![
                session("019f0000-old", Some(7), "thinking"),
                session("019f0001-new", Some(7), "idle"),
            ]),
            vec![session("019f0001-new", Some(7), "idle")]
        );
    }

    #[test]
    fn latest_sessions_by_window_preserves_unbound_sessions() {
        assert_eq!(
            latest_sessions_by_window(vec![
                session("019f0000-old", Some(7), "thinking"),
                session("019f0001-new", Some(7), "idle"),
                session("subagent", None, "thinking"),
            ]),
            vec![
                session("subagent", None, "thinking"),
                session("019f0001-new", Some(7), "idle"),
            ]
        );
    }

    #[test]
    fn agent_session_children_filters_method_call_files() {
        assert_eq!(
            agent_session_children(vec![
                LocusPath::new("/dbus/session/io/github/AgentDBus/sessions/codex/session-one"),
                LocusPath::new(
                    "/dbus/session/io/github/AgentDBus/sessions/codex/RespondToElicitation.call",
                ),
                LocusPath::new(
                    "/dbus/session/io/github/AgentDBus/sessions/codex/io.github.AgentDBus1.Session.RespondToElicitationById.call",
                ),
            ]),
            vec![LocusPath::new(
                "/dbus/session/io/github/AgentDBus/sessions/codex/session-one",
            )]
        );
    }
}
