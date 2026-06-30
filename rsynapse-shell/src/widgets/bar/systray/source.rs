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
    pub(super) passive: bool,
    pub(super) icon: String,
    pub(super) icon_pixmap: Option<TrayIconPixmap>,
    pub(super) tooltip: String,
    pub(super) needs_attention: bool,
    pub(super) menu: Option<LocusPath>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TrayIconPixmap {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) argb32_hex: String,
}

impl Default for TrayItemVm {
    fn default() -> Self {
        Self {
            visible: false,
            passive: false,
            icon: FALLBACK_ICON.to_owned(),
            icon_pixmap: None,
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
    pub(super) toggle_type: String,
    pub(super) toggle_state: i32,
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
            toggle_type: String::new(),
            toggle_state: -1,
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
        item.observe_prop_or::<String>("menu-path", String::new()),
        tray_item_pixmap(item.clone())
            => move |(state, title, status, icon, attention_icon, category, service, menu_path, icon_pixmap)| {
                match state {
                    NodeState::Present => tray_item_view(
                        title,
                        status,
                        icon,
                        attention_icon,
                        category,
                        service,
                        menu_path,
                        icon_pixmap,
                    ),
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
    icon_pixmap: Option<TrayIconPixmap>,
) -> TrayItemVm {
    let passive = status.trim() == "Passive";
    let needs_attention = status == "NeedsAttention";
    let icon = if needs_attention && !attention_icon.trim().is_empty() {
        attention_icon
    } else {
        icon
    };
    let icon = non_empty(icon.as_str()).unwrap_or(FALLBACK_ICON).to_owned();
    let tooltip = tooltip(title.as_str(), status.as_str(), category.as_str());

    TrayItemVm {
        visible: true,
        passive,
        icon,
        icon_pixmap,
        tooltip,
        needs_attention,
        menu: dbusmenu_path(service.as_str(), menu_path.as_str()),
    }
}

fn tray_item_pixmap(item: LocusPath) -> Observable<Option<TrayIconPixmap>> {
    combine_latest!(
        item.observe_prop_or::<u32>("icon-pixmap-width", 0),
        item.observe_prop_or::<u32>("icon-pixmap-height", 0),
        item.observe_prop_or::<String>("icon-pixmap-argb32", String::new())
            => |(width, height, argb32_hex)| tray_icon_pixmap(width, height, argb32_hex),
    )
    .distinct_until_changed()
    .box_it()
}

fn tray_icon_pixmap(width: u32, height: u32, argb32_hex: String) -> Option<TrayIconPixmap> {
    let argb32_hex = non_empty(argb32_hex.as_str())?;
    (width > 0 && height > 0).then(|| TrayIconPixmap {
        width,
        height,
        argb32_hex: argb32_hex.to_owned(),
    })
}

pub(super) fn tray_menu_items(menu: LocusPath) -> Observable<Vec<LocusPath>> {
    menu.child("item")
        .as_children()
        .map(sort_paths)
        .distinct_until_changed()
        .box_it()
}

pub(super) fn tray_menu_item_vm(item: LocusPath) -> Observable<TrayMenuItemVm> {
    combine_latest!(
        item.observe_prop_or::<String>("label", String::new()),
        item.observe_prop_or::<bool>("enabled", true),
        item.observe_prop_or::<bool>("visible", true),
        item.observe_prop_or::<String>("type", String::new()),
        item.observe_prop_or::<u32>("position", u32::MAX),
        item.observe_prop_or::<String>("toggle-type", String::new()),
        item.observe_prop_or::<i32>("toggle-state", -1)
            => move |(label, enabled, visible, item_type, position, toggle_type, toggle_state)| {
                TrayMenuItemVm {
                    path: item.clone(),
                    visible,
                    label: clean_menu_label(label.as_str()),
                    enabled,
                    separator: item_type == "separator",
                    position,
                    toggle_type,
                    toggle_state,
                }
            },
    )
    .distinct_until_changed()
    .box_it()
}

pub(super) fn tray_menu_item_children(item: LocusPath) -> Observable<Vec<LocusPath>> {
    item.child("child")
        .as_children()
        .map(sort_paths)
        .distinct_until_changed()
        .box_it()
}

pub(super) fn clean_menu_label(label: &str) -> String {
    let mut cleaned = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(char) = chars.next() {
        if char != '_' {
            cleaned.push(char);
            continue;
        }

        match chars.peek().copied() {
            Some('_') => {
                cleaned.push('_');
                chars.next();
            }
            Some(next) if !next.is_whitespace() => {
                cleaned.push(next);
                chars.next();
            }
            _ => cleaned.push(char),
        }
    }
    cleaned
}

fn sort_paths(mut paths: Vec<LocusPath>) -> Vec<LocusPath> {
    paths.sort_by(|left, right| left.as_path().cmp(right.as_path()));
    paths
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

pub(super) fn dbusmenu_path(service: &str, menu_path: &str) -> Option<LocusPath> {
    let service = non_empty(service)?;
    let menu_path = non_empty(menu_path)?;
    Some(
        source::root()
            .child("dbusmenu/menu")
            .encoded_child(dbusmenu_local_id(service, menu_path)),
    )
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
