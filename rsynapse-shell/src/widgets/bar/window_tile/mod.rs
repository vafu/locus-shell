mod source;

use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use self::source::{WindowTileKind, WindowTileVm, window_tile_vm};
use super::WindowNode;

#[derive(Debug)]
#[shell_macros::model(module = window_tile_sources)]
pub(super) struct WindowTile {
    pub window: WindowNode,

    #[source(window_tile_vm(window.clone()))]
    pub vm: Option<WindowTileVm>,
}

#[shell_macros::component(
    module = window_tile_sources,
    model = WindowTile
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for WindowTile {
    type Init = WindowNode;
    type Input = window_tile_sources::Msg;
    type Output = ();

    view! {
        gtk::Box {
            #[watch]
            set_visible: model.vm.is_some(),

            #[watch]
            set_css_classes: &window_tile_classes(&model.vm),

            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Fill,
            set_vexpand: true,

            #[watch]
            set_tooltip_text: model.vm.as_ref().map(|vm| vm.tooltip.as_str()),

            gtk::Box {
                add_css_class: "workspace-window-content",
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Fill,
                set_vexpand: true,

                gtk::Image {
                    #[watch]
                    set_icon_name: window_icon_name(&model.vm).as_deref(),
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = WindowTile::new(init);
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

fn window_tile_classes(vm: &Option<WindowTileVm>) -> Vec<&'static str> {
    let Some(vm) = vm else {
        return vec![
            "workspace-window-frame",
            "workspace-window-tile",
            "workspace-window-plain",
        ];
    };

    let mut classes = vec!["workspace-window-frame", "workspace-window-tile"];
    classes.push(match vm.kind {
        WindowTileKind::Plain => "workspace-window-plain",
        WindowTileKind::Neovim => "workspace-window-neovim",
    });

    if vm.active {
        classes.push("active");
    }
    if vm.urgent {
        classes.push("urgent");
    }

    classes
}

fn window_icon_name(vm: &Option<WindowTileVm>) -> Option<String> {
    vm.as_ref().map(|vm| vm.icon.clone())
}
