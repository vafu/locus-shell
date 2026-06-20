mod audio;
mod battery;
mod locus;
mod network;
mod time;
mod windows;

pub(crate) use audio::{AudioView, audio_status};
pub(crate) use battery::{BatteryView, battery_status};
pub(crate) use locus::{
    WindowNode, WorkspaceNode, selected_workspace_windows, workspace_active, workspace_index,
    workspace_name, workspace_project_branch, workspace_project_icon, workspace_project_name,
    workspace_urgent, workspaces,
};
pub(crate) use network::{NetworkView, network_status};
pub(crate) use time::{ClockView, clock};
pub(crate) use windows::{WindowTileKind, WindowTileVm, window_tile_vm};
