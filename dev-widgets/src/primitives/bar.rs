use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    list::ComponentListBoxExt,
    window::{self, Anchors, Edge, Layer, WindowConfig},
};

use crate::locus::{self, WindowNode};

use super::window_title::WindowTitle;

pub struct BarInit {
    pub title: &'static str,
}

#[shell_macros::model]
pub struct Bar {
    #[source(locus::selected_workspace_windows())]
    pub window_nodes: Vec<WindowNode>,
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
