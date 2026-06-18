mod source;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowTileKind {
    #[default]
    Plain,
    Agent,
    Neovim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowTileView {
    pub kind: WindowTileKind,
    pub icon: String,
    pub tooltip: String,
    pub urgent: bool,
    pub context_pct: u32,
    pub substatus_count: u32,
}

impl Default for WindowTileView {
    fn default() -> Self {
        Self {
            kind: WindowTileKind::Plain,
            icon: "application-x-executable-symbolic".to_owned(),
            tooltip: String::new(),
            urgent: false,
            context_pct: 0,
            substatus_count: 0,
        }
    }
}

pub(crate) use source::window_tile_for_window;
