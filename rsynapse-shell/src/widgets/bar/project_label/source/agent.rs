use std::collections::BTreeSet;

use shell_core::{
    locus_path::LocusPath,
    source::{Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use crate::widgets::bar::{agent_sessions::agent_sessions, window_source::window_snapshots};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::widgets::bar::project_label) struct WorkspaceAgentState {
    pub(in crate::widgets::bar::project_label) has_attention: bool,
    pub(in crate::widgets::bar::project_label) has_working: bool,
}

pub(super) fn workspace_agent_state(workspace: LocusPath) -> Observable<WorkspaceAgentState> {
    combine_latest!(
        workspace.observe_prop_or::<u32>("id", u32::MAX),
        window_snapshots(),
        agent_sessions()
            => |(workspace_id, windows, sessions)| {
                let workspace_window_ids = windows
                    .iter()
                    .filter(|window| window.workspace_id == Some(workspace_id))
                    .map(|window| window.id)
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

fn is_working_status(status: &str) -> bool {
    let status = status.to_ascii_lowercase();
    status.contains("compact")
        || status.contains("tool")
        || status.contains("thinking")
        || status.contains("working")
        || status.contains("running")
        || status.contains("busy")
}
