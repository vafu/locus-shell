use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) struct WindowSnapshot {
    pub(in crate::widgets::bar) path: LocusPath,
    pub(in crate::widgets::bar) workspace_id: Option<u32>,
    pub(in crate::widgets::bar) column: u32,
    pub(in crate::widgets::bar) row: u32,
    pub(in crate::widgets::bar) id: u32,
    pub(in crate::widgets::bar) app_id: Option<String>,
}

pub(in crate::widgets::bar) fn window_snapshots() -> Observable<Vec<WindowSnapshot>> {
    source::shared_by_key("rsynapse.window-snapshots", "all", || {
        source::root()
            .child("window")
            .as_children()
            .switch_map(|windows| {
                source::combine_latest_vec(windows.into_iter().map(window_snapshot).collect())
            })
            .map(|windows| windows)
            .distinct_until_changed()
            .box_it()
    })
}

fn window_snapshot(window: LocusPath) -> Observable<WindowSnapshot> {
    combine_latest!(
        window.observe_prop::<u32>("workspace-id"),
        window.observe_prop_or::<u32>("column", u32::MAX),
        window.observe_prop_or::<u32>("row", u32::MAX),
        window.observe_prop_or::<u32>("id", u32::MAX),
        window.observe_prop_or::<String>("app-id", String::new()).map(non_empty)
            => move |(workspace_id, column, row, id, app_id)| WindowSnapshot {
                path: window.clone(),
                workspace_id,
                column,
                row,
                id,
                app_id,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}
