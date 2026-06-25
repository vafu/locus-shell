use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use super::non_empty;
use crate::desktop_icon;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct WorkspaceFallback {
    pub(super) icon: Option<String>,
    pub(super) empty: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceWindowModel {
    path: LocusPath,
    workspace_id: Option<u32>,
    column: u32,
    row: u32,
    id: u32,
    icon: Option<String>,
}

pub(super) fn workspace_window_fallback_source(
    workspace: LocusPath,
) -> Observable<WorkspaceFallback> {
    let workspace_id = workspace.observe_prop_or::<u32>("id", u32::MAX).map(Some);
    let windows = source::root()
        .child("window")
        .as_children()
        .switch_map(|windows| {
            source::combine_latest_vec(windows.into_iter().map(workspace_window_model).collect())
        });

    workspace_id
        .combine_latest(windows, workspace_window_fallback)
        .distinct_until_changed()
        .box_it()
}

fn workspace_window_model(window: LocusPath) -> Observable<WorkspaceWindowModel> {
    combine_latest!(
        window.observe_prop_or::<u32>("workspace-id", u32::MAX).map(Some),
        window.observe_prop_or::<u32>("column", u32::MAX),
        window.observe_prop_or::<u32>("row", u32::MAX),
        window.observe_prop_or::<u32>("id", u32::MAX),
        window.observe_prop_or::<String>("app-id", String::new()).map(non_empty_value)
            => move |(workspace_id, column, row, id, app_id)| WorkspaceWindowModel {
                path: window.clone(),
                workspace_id,
                column,
                row,
                id,
                icon: app_id.as_deref().and_then(app_icon),
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn workspace_window_fallback(
    workspace_id: Option<u32>,
    mut windows: Vec<WorkspaceWindowModel>,
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
        icon: windows.into_iter().find_map(|window| window.icon),
        empty,
    }
}

fn app_icon(app_id: &str) -> Option<String> {
    let desktop_icon = non_empty(Some(desktop_icon::icon_for_app_id(app_id)));
    desktop_icon.or_else(|| non_empty(Some(app_id.to_owned())))
}

fn non_empty_value(value: String) -> Option<String> {
    non_empty(Some(value))
}
