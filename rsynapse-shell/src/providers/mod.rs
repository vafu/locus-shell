mod project_labels;
mod windows;

pub(crate) use project_labels::ProjectLabelView;
pub(crate) use project_labels::project_label_for_workspace;
pub(crate) use windows::{WindowTileKind, WindowTileView, window_tile_for_window};
