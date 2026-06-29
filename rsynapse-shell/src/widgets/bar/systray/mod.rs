mod source;

#[cfg(test)]
mod test;

use std::{cell::RefCell, fs, rc::Rc, thread};

use relm4::prelude::*;
use relm4::{Controller, component::ComponentController};
use shell_core::{
    gtk::{self, prelude::*},
    list::ComponentListBoxExt,
    locus_path::LocusPath,
};

use self::source::{
    TrayIconPixmap, TrayItemVm, TrayMenuItemVm, tray_item_vm, tray_menu_item_vm, tray_menu_items,
};

pub(super) use self::source::tray_items as systray_items;

#[derive(Debug)]
#[shell_macros::model(module = tray_item_sources)]
pub(super) struct TrayItem {
    pub item: LocusPath,

    #[source(tray_item_vm(item.clone()))]
    pub vm: TrayItemVm,
}

#[shell_macros::component(
    module = tray_item_sources,
    model = TrayItem
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for TrayItem {
    type Init = LocusPath;
    type Input = tray_item_sources::Msg;
    type Output = ();

    view! {
        gtk::MenuButton {
            #[watch]
            set_css_classes: &tray_item_classes(&model.vm),
            #[watch]
            set_visible: model.vm.visible,
            #[watch]
            set_tooltip_text: Some(model.vm.tooltip.as_str()),

            #[wrap(Some)]
            set_child = &gtk::Image {
                add_css_class: "tray-icon",
                #[watch]
                set_from_gicon: &tray_icon(&model.vm),
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = TrayItem::new(init);
        let widgets = view_output!();
        let menu_popover = gtk::Popover::new();
        menu_popover.add_css_class("menu");
        menu_popover.add_css_class("tray-menu-popover");
        let menu_mount = gtk::Box::new(gtk::Orientation::Vertical, 0);
        menu_mount.add_css_class("tray-menu");
        menu_popover.set_child(Some(&menu_mount));
        root.set_popover(Some(&menu_popover));
        let menu_controller = Rc::new(RefCell::new(None));
        mount_tray_menu(
            &menu_popover,
            &menu_mount,
            &menu_controller,
            model.item.clone(),
        );
        let item = model.item.clone();
        let menu_controller = menu_controller.clone();
        let menu_popover_for_signal = menu_popover.clone();
        let menu_mount_for_signal = menu_mount.clone();
        menu_popover.connect_visible_notify(move |_| {
            mount_tray_menu(
                &menu_popover_for_signal,
                &menu_mount_for_signal,
                &menu_controller,
                item.clone(),
            );
        });

        ComponentParts { model, widgets }
    }
}

#[derive(Debug)]
#[shell_macros::model(module = tray_menu_sources)]
pub(crate) struct TrayMenu {
    pub item: LocusPath,

    #[source(tray_menu_items(item.clone()))]
    pub items: Vec<LocusPath>,
}

#[shell_macros::component(
    module = tray_menu_sources,
    model = TrayMenu
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for TrayMenu {
    type Init = LocusPath;
    type Input = tray_menu_sources::Msg;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            add_css_class: "tray-menu",
            set_orientation: gtk::Orientation::Vertical,

            #[bind_list(items, row = TrayMenuItem)]
            items -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = TrayMenu::new(init);
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

#[derive(Debug)]
#[shell_macros::model(module = tray_menu_item_sources)]
pub(crate) struct TrayMenuItem {
    pub item: LocusPath,

    #[source(tray_menu_item_vm(item.clone()))]
    pub vm: TrayMenuItemVm,
}

#[shell_macros::component(
    module = tray_menu_item_sources,
    model = TrayMenuItem
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for TrayMenuItem {
    type Init = LocusPath;
    type Input = tray_menu_item_sources::Msg;
    type Output = ();

    view! {
        #[root]
        gtk::Button {
            add_css_class: "flat",
            add_css_class: "tray-menu-row",
            #[watch]
            set_sensitive: model.vm.enabled,
            #[watch]
            set_visible: model.vm.visible && !model.vm.separator,

            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: model.vm.label.as_str(),
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = TrayMenuItem::new(init);
        let widgets = view_output!();
        let item = model.item.clone();
        let root_button = root.clone();

        root.connect_clicked(move |_| {
            if let Some(popover) = root_button
                .ancestor(gtk::Popover::static_type())
                .and_then(|widget| widget.downcast::<gtk::Popover>().ok())
            {
                popover.popdown();
            }

            activate_menu_item(item.clone());
        });

        ComponentParts { model, widgets }
    }
}

fn tray_item_classes(vm: &TrayItemVm) -> Vec<&'static str> {
    let mut classes = vec!["flat", "circular", "tray-item"];
    if vm.needs_attention {
        classes.push("needs-attention");
    }
    classes
}

fn tray_icon(vm: &TrayItemVm) -> gtk::gio::Icon {
    if let Some(texture) = tray_pixmap_texture(vm.icon_pixmap.as_ref()) {
        return texture.upcast();
    }

    let names = tray_icon_names(vm.icon.as_str());
    let names = names.iter().map(String::as_str).collect::<Vec<_>>();
    gtk::gio::ThemedIcon::from_names(&names).upcast()
}

fn tray_icon_names(icon: &str) -> Vec<String> {
    let mut names = Vec::new();
    push_icon_name(&mut names, icon);
    for alias in tray_icon_aliases(icon) {
        push_icon_name(&mut names, alias);
    }
    if let Some(symbolic_base) = icon.strip_suffix("-symbolic") {
        push_icon_name(&mut names, symbolic_base);
    } else if !icon.is_empty() {
        push_icon_name(&mut names, format!("{icon}-symbolic"));
    }
    push_icon_name(&mut names, "application-x-executable-symbolic");
    names
}

fn tray_icon_aliases(icon: &str) -> &'static [&'static str] {
    match icon {
        "nm-device-wired" | "nm-device-wired-secure" => {
            &["network-wired-symbolic", "network-wired"]
        }
        "nm-device-wired-autoip" | "nm-device-wired-acquiring" => {
            &["network-wired-acquiring-symbolic", "network-wired"]
        }
        "nm-device-wired-disconnected" => &[
            "network-wired-disconnected-symbolic",
            "network-offline-symbolic",
        ],
        "nm-no-connection" | "nm-no-connection-symbolic" => {
            &["network-offline-symbolic", "network-error-symbolic"]
        }
        "nm-vpn-active-lock" | "nm-vpn-lock" => &["network-vpn-symbolic"],
        "nm-signal-100" | "nm-signal-100-secure" => &["network-wireless-signal-excellent-symbolic"],
        "nm-signal-75" | "nm-signal-75-secure" => &["network-wireless-signal-good-symbolic"],
        "nm-signal-50" | "nm-signal-50-secure" => &["network-wireless-signal-ok-symbolic"],
        "nm-signal-25" | "nm-signal-25-secure" => &["network-wireless-signal-weak-symbolic"],
        "nm-signal-00" | "nm-signal-0" | "nm-signal-00-secure" => {
            &["network-wireless-signal-none-symbolic"]
        }
        _ => &[],
    }
}

fn push_icon_name(names: &mut Vec<String>, name: impl AsRef<str>) {
    let name = name.as_ref().trim();
    if !name.is_empty() && !names.iter().any(|existing| existing == name) {
        names.push(name.to_owned());
    }
}

fn tray_pixmap_texture(pixmap: Option<&TrayIconPixmap>) -> Option<gtk::gdk::MemoryTexture> {
    let pixmap = pixmap?;
    let bytes = decode_hex(pixmap.argb32_hex.as_str())?;
    let stride = pixmap.width.checked_mul(4)? as usize;
    let expected = stride.checked_mul(pixmap.height as usize)?;
    if bytes.len() != expected {
        return None;
    }

    let bytes = gtk::glib::Bytes::from_owned(bytes);
    Some(gtk::gdk::MemoryTexture::new(
        pixmap.width.try_into().ok()?,
        pixmap.height.try_into().ok()?,
        gtk::gdk::MemoryFormat::B8g8r8a8,
        &bytes,
        stride,
    ))
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    let value = value.trim();
    if value.len() % 2 != 0 {
        return None;
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Some((hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?))
        .collect()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn activate_menu_item(item: LocusPath) {
    thread::spawn(move || {
        if let Err(error) = fs::write(item.prop("activate").as_path(), "true") {
            eprintln!("[systray] failed to activate DBusMenu item: {error}");
        }
    });
}

fn mount_tray_menu(
    popover: &gtk::Popover,
    mount: &gtk::Box,
    controller: &Rc<RefCell<Option<Controller<TrayMenu>>>>,
    item: LocusPath,
) {
    if popover.is_visible() {
        if controller.borrow().is_none() {
            let launched = TrayMenu::builder().launch(item).detach();
            let widget = <gtk::Box as AsRef<gtk::Widget>>::as_ref(launched.widget()).clone();
            mount.append(&widget);
            *controller.borrow_mut() = Some(launched);
        }
        return;
    }

    if let Some(launched) = controller.borrow_mut().take() {
        let widget = <gtk::Box as AsRef<gtk::Widget>>::as_ref(launched.widget()).clone();
        mount.remove(&widget);
    }
}
