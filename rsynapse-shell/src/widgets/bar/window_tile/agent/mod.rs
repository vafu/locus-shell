mod source;

pub(super) use source::agent_for_window;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) struct Agent {
    pub(super) icon: String,
    pub(super) attention: bool,
    pub(super) state: State,
    pub(super) context_pct: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::widgets::bar) enum State {
    #[default]
    None,
    Thinking,
    ToolUse,
    Compacting,
}
