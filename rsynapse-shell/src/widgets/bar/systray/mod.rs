mod source;

use std::{fs, thread};

use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    list::ComponentListBoxExt,
    locus_path::LocusPath,
};

use self::source::{TrayItemVm, TrayMenuItemVm, tray_item_vm, tray_menu_item_vm, tray_menu_items};

pub(super) use self::source::tray_items as systray_items;

#[derive(Debug)]
#[shell_macros::model(module = tray_item_sources)]
pub(super) struct TrayItem {
    pub item: LocusPath,

    #[source(tray_item_vm(item.clone()))]
    pub vm: TrayItemVm,

    #[source(tray_menu_items(item.clone()))]
    pub menu_items: Vec<LocusPath>,
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
            set_popover = &gtk::Popover {
                add_css_class: "menu",
                add_css_class: "tray-menu-popover",

                #[bind_list(menu_items, row = TrayMenuItem)]
                menu_items -> gtk::Box {
                    add_css_class: "tray-menu",
                    set_orientation: gtk::Orientation::Vertical,
                }
            },

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

fn tray_icon(vm: &TrayItemVm) -> gtk::gio::ThemedIcon {
    let mut names = vec![vm.icon.as_str()];
    let symbolic_base = vm.icon.strip_suffix("-symbolic");
    if let Some(icon) = symbolic_base {
        names.push(icon);
    }
    if vm.icon != "application-x-executable-symbolic" {
        names.push("application-x-executable-symbolic");
    }
    gtk::gio::ThemedIcon::from_names(&names)
}

fn activate_menu_item(item: LocusPath) {
    thread::spawn(move || {
        if let Err(error) = fs::write(item.prop("activate").as_path(), "true") {
            eprintln!("[systray] failed to activate DBusMenu item: {error}");
        }
    });
}
