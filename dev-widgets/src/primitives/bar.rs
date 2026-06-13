use common_providers::upower::{DISPLAY_DEVICE, DisplayDevice};
use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    list::ComponentListBoxExt,
    window::{self, Anchors, Edge, Layer, WindowConfig},
};

use crate::locus_schema::{self, WorkspacePathExt};

use super::{
    battery::battery_fraction,
    window_title::{WindowNode, WindowTitle},
};

pub struct BarInit {
    pub title: &'static str,
}

#[shell_macros::model]
pub struct Bar {
    #[source(locus_schema::paths::SELECTED_WORKSPACE.windows())]
    pub window_nodes: Vec<WindowNode>,

    #[source(DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE))]
    pub battery_percent: f64,
}

#[shell_macros::component(model = Bar)]
#[relm4::component(pub)]
impl SimpleComponent for Bar {
    type Init = BarInit;
    type Input = sources::Msg;
    type Output = ();

    view! {
        gtk::Window {
            gtk::Box {
                set_widget_name: "dev-bar",
                add_css_class: "dev-panel",
                set_orientation: gtk::Orientation::Horizontal,

                #[bind_list(window_nodes, row = WindowTitle)]
                window_list -> gtk::Box {
                    set_widget_name: "workspace-window-list",
                    add_css_class: "dev-panel__window-list",
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                },

                gtk::ProgressBar {
                    set_widget_name: "battery-percent",
                    add_css_class: "dev-panel__battery",
                    set_show_text: true,

                    #[bind(battery_percent)]
                    set_fraction: |percent| battery_fraction(percent),
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        window::apply_layer_shell_config(&root, bar_window_config());
        root.set_title(Some(init.title));

        let model = Bar::default();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

fn bar_window_config() -> WindowConfig {
    WindowConfig::new(Layer::Top)
        .with_anchors(
            Anchors::NONE
                .with_edge(Edge::Top)
                .with_edge(Edge::Right)
                .with_edge(Edge::Left),
        )
        .with_auto_exclusive_zone()
        .with_namespace("locus-dev-bar")
}
