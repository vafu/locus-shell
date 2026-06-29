use shell_core::{
    locus_path::LocusPath,
    source::{self, NodeState, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

const TRAY_ITEMS_PATH: &str = "statusnotifier/item";
const FALLBACK_ICON: &str = "application-x-executable-symbolic";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrayItemVm {
    pub(super) visible: bool,
    pub(super) icon: String,
    pub(super) tooltip: String,
    pub(super) needs_attention: bool,
    pub(super) menu: Option<LocusPath>,
}

impl Default for TrayItemVm {
    fn default() -> Self {
        Self {
            visible: false,
            icon: FALLBACK_ICON.to_owned(),
            tooltip: String::new(),
            needs_attention: false,
            menu: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrayMenuItemVm {
    pub(super) path: LocusPath,
    pub(super) visible: bool,
    pub(super) label: String,
    pub(super) enabled: bool,
    pub(super) separator: bool,
    pub(super) position: u32,
}

impl Default for TrayMenuItemVm {
    fn default() -> Self {
        Self {
            path: source::root(),
            visible: false,
            label: String::new(),
            enabled: false,
            separator: false,
            position: u32::MAX,
        }
    }
}

pub(crate) fn tray_items() -> Observable<Vec<LocusPath>> {
    source::shared_by_key("rsynapse.tray-items", TRAY_ITEMS_PATH, || {
        source::root()
            .child(TRAY_ITEMS_PATH)
            .as_children()
            .map(|mut items| {
                items.sort_by(|left, right| left.as_path().cmp(right.as_path()));
                items
            })
            .distinct_until_changed()
            .box_it()
    })
}

pub(super) fn tray_item_vm(item: LocusPath) -> Observable<TrayItemVm> {
    combine_latest!(
        item.as_node(),
        item.observe_prop_or::<String>("title", String::new()),
        item.observe_prop_or::<String>("status", String::new()),
        item.observe_prop_or::<String>("icon-name", String::new()),
        item.observe_prop_or::<String>("attention-icon-name", String::new()),
        item.observe_prop_or::<String>("category", String::new()),
        item.observe_prop_or::<String>("service-name", String::new()),
        item.observe_prop_or::<String>("menu-path", String::new())
            => move |(state, title, status, icon, attention_icon, category, service, menu_path)| {
                match state {
                    NodeState::Present => tray_item_view(title, status, icon, attention_icon, category, service, menu_path),
                    NodeState::Missing => TrayItemVm::default(),
                }
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn tray_item_view(
    title: String,
    status: String,
    icon: String,
    attention_icon: String,
    category: String,
    service: String,
    menu_path: String,
) -> TrayItemVm {
    let needs_attention = status == "NeedsAttention";
    let icon = if needs_attention && !attention_icon.trim().is_empty() {
        attention_icon
    } else {
        icon
    };
    let icon = non_empty(icon.as_str()).unwrap_or(FALLBACK_ICON).to_owned();
    let tooltip = tooltip(title.as_str(), status.as_str(), category.as_str());

    TrayItemVm {
        visible: status.trim() != "Passive",
        icon,
        tooltip,
        needs_attention,
        menu: dbusmenu_path(service.as_str(), menu_path.as_str()),
    }
}

pub(super) fn tray_menu_items(item: LocusPath) -> Observable<Vec<LocusPath>> {
    combine_latest!(
        item.observe_prop_or::<String>("service-name", String::new()),
        item.observe_prop_or::<String>("menu-path", String::new()),
        dbusmenu_items()
            => |(service, menu_path, items)| dbusmenu_items_for_menu(service.as_str(), menu_path.as_str(), items),
    )
    .distinct_until_changed()
    .box_it()
}

fn dbusmenu_items() -> Observable<Vec<LocusPath>> {
    source::shared_by_key("rsynapse.dbusmenu-items", "all", || {
        source::root().child("dbusmenu/item").as_children()
    })
}

pub(super) fn tray_menu_item_vm(item: LocusPath) -> Observable<TrayMenuItemVm> {
    combine_latest!(
        item.observe_prop_or::<String>("label", String::new()),
        item.observe_prop_or::<bool>("enabled", true),
        item.observe_prop_or::<bool>("visible", true),
        item.observe_prop_or::<String>("type", String::new()),
        item.observe_prop_or::<u32>("position", u32::MAX)
            => move |(label, enabled, visible, item_type, position)| {
                TrayMenuItemVm {
                    path: item.clone(),
                    visible,
                    label,
                    enabled,
                    separator: item_type == "separator",
                    position,
                }
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn tooltip(title: &str, status: &str, category: &str) -> String {
    [non_empty(title), non_empty(status), non_empty(category)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn dbusmenu_path(service: &str, menu_path: &str) -> Option<LocusPath> {
    let service = non_empty(service)?;
    let menu_path = non_empty(menu_path)?;
    Some(
        source::root()
            .child("dbusmenu/menu")
            .encoded_child(dbusmenu_local_id(service, menu_path)),
    )
}

fn dbusmenu_items_for_menu(
    service: &str,
    menu_path: &str,
    mut items: Vec<LocusPath>,
) -> Vec<LocusPath> {
    let Some(service) = non_empty(service) else {
        return Vec::new();
    };
    let Some(menu_path) = non_empty(menu_path) else {
        return Vec::new();
    };
    let prefix = format!("{}:", dbusmenu_local_id(service, menu_path));
    items.retain(|item| {
        item.as_path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix.as_str()))
    });
    items.sort_by(|left, right| left.as_path().cmp(right.as_path()));
    items
}

fn dbusmenu_local_id(service: &str, path: &str) -> String {
    let service = service.trim_start_matches(':').replace(['.', '/'], "_");
    let path = path.trim_start_matches('/').replace('/', "_");
    if path.is_empty() {
        service
    } else {
        format!("{service}:{path}")
    }
}
