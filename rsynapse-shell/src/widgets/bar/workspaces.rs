use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceEntry {
    path: LocusPath,
    index: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WindowEntry {
    path: LocusPath,
    workspace_id: Option<u32>,
    column: u32,
    row: u32,
    id: u32,
}

pub(super) fn workspaces() -> Observable<Vec<LocusPath>> {
    source::root()
        .child("workspace")
        .as_children()
        .switch_map(|workspaces| {
            source::combine_latest_vec(workspaces.into_iter().map(workspace_entry).collect())
        })
        .map(|mut workspaces| {
            workspaces.sort_by(|left, right| {
                left.index
                    .cmp(&right.index)
                    .then_with(|| left.path.as_path().cmp(right.path.as_path()))
            });
            workspaces
                .into_iter()
                .map(|workspace| workspace.path)
                .collect()
        })
        .distinct_until_changed()
        .box_it()
}

fn workspace_entry(workspace: LocusPath) -> Observable<WorkspaceEntry> {
    workspace
        .observe_prop_or::<u32>("index", u32::MAX)
        .map(move |index| WorkspaceEntry {
            path: workspace.clone(),
            index,
        })
        .distinct_until_changed()
        .box_it()
}

fn selected_workspace() -> Observable<LocusPath> {
    source::root()
        .child("context/selected")
        .observe_rel("workspace")
        .filter_map(|workspace| workspace)
        .distinct_until_changed()
        .box_it()
}

pub(super) fn selected_workspace_windows() -> Observable<Vec<LocusPath>> {
    let windows = source::root()
        .child("window")
        .as_children()
        .switch_map(|windows| {
            source::combine_latest_vec(windows.into_iter().map(window_entry).collect())
        });

    selected_workspace()
        .map(|workspace| {
            workspace
                .as_path()
                .file_name()
                .and_then(|value| value.to_str())
                .and_then(|value| value.parse::<u32>().ok())
        })
        .combine_latest(windows, |selected_workspace_id, mut windows| {
            let Some(selected_workspace_id) = selected_workspace_id else {
                return Vec::new();
            };

            windows.retain(|window| window.workspace_id == Some(selected_workspace_id));
            windows.sort_by(|left, right| {
                (left.column, left.row, left.id)
                    .cmp(&(right.column, right.row, right.id))
                    .then_with(|| left.path.as_path().cmp(right.path.as_path()))
            });
            windows.into_iter().map(|window| window.path).collect()
        })
        .distinct_until_changed()
        .box_it()
}

fn window_entry(window: LocusPath) -> Observable<WindowEntry> {
    shell_rx_macros::combine_latest!(
        window.observe_prop::<u32>("workspace-id"),
        window.observe_prop_or::<u32>("column", u32::MAX),
        window.observe_prop_or::<u32>("row", u32::MAX),
        window.observe_prop_or::<u32>("id", u32::MAX)
            => move |(workspace_id, column, row, id)| WindowEntry {
                path: window.clone(),
                workspace_id,
                column,
                row,
                id,
            },
    )
    .distinct_until_changed()
    .box_it()
}
