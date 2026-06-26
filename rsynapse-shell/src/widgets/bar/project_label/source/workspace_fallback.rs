use shell_core::{locus_path::LocusPath, source::Observable, source::rx::Observable as _};

use super::non_empty;
use crate::{
    desktop_icon,
    widgets::bar::window_source::{WindowSnapshot, window_snapshots},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct WorkspaceFallback {
    pub(super) icon: Option<String>,
    pub(super) empty: bool,
}

pub(super) fn workspace_window_fallback_source(
    workspace: LocusPath,
) -> Observable<WorkspaceFallback> {
    let workspace_id = workspace.observe_prop_or::<u32>("id", u32::MAX).map(Some);

    workspace_id
        .combine_latest(window_snapshots(), workspace_window_fallback)
        .distinct_until_changed()
        .box_it()
}

fn workspace_window_fallback(
    workspace_id: Option<u32>,
    mut windows: Vec<WindowSnapshot>,
) -> WorkspaceFallback {
    let Some(workspace_id) = workspace_id else {
        return WorkspaceFallback {
            icon: None,
            empty: true,
        };
    };

    windows.retain(|window| window.workspace_id == Some(workspace_id));
    windows.sort_by(|left, right| {
        (left.column, left.row, left.id)
            .cmp(&(right.column, right.row, right.id))
            .then_with(|| left.path.as_path().cmp(right.path.as_path()))
    });
    let empty = windows.is_empty();

    WorkspaceFallback {
        icon: windows
            .into_iter()
            .filter_map(|window| window.app_id)
            .find_map(|app_id| app_icon(&app_id)),
        empty,
    }
}

fn app_icon(app_id: &str) -> Option<String> {
    let desktop_icon = non_empty(Some(desktop_icon::icon_for_app_id(app_id)));
    desktop_icon.or_else(|| non_empty(Some(app_id.to_owned())))
}
