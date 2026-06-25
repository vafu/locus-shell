use shell_core::source::{Observable, rx::Observable as _};
use shell_rx_macros::combine_latest;

use crate::desktop_icon;

use super::super::WindowNode;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::widgets::bar) enum WindowTileKind {
    #[default]
    Plain,
    Neovim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) struct WindowTileVm {
    pub(super) kind: WindowTileKind,
    pub(super) icon: String,
    pub(super) tooltip: String,
    pub(super) active: bool,
    pub(super) urgent: bool,
}

pub(super) fn window_tile_vm(window: WindowNode) -> Observable<Option<WindowTileVm>> {
    let title = window.observe_prop::<String>("title").map(non_empty);
    let app_id = window.observe_prop::<String>("app-id").map(non_empty);
    let active = window.observe_prop_or::<bool>("selected", false);
    let urgent = window.observe_prop_or::<bool>("urgent", false);

    combine_latest!(
        title,
        app_id,
        active,
        urgent
            => |(title, app_id, active, urgent)| {
                let app_id = app_id.unwrap_or_default();
                let title = title.unwrap_or_default();
                Some(WindowTileVm {
                    kind: window_kind(&title),
                    icon: desktop_icon::icon_for_app_id(&app_id),
                    tooltip: title,
                    active,
                    urgent,
                })
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn window_kind(title: &str) -> WindowTileKind {
    let title = title.to_ascii_lowercase();
    if title.contains("nvim") || title.contains("neovim") {
        WindowTileKind::Neovim
    } else {
        WindowTileKind::Plain
    }
}

pub(super) fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}
