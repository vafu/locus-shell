mod project_label;
mod window_tile;

use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    list::ComponentListBoxExt,
    window::{self, Anchors, Edge, Layer, WindowConfig},
};

use locus_provider::NodeRef;

use crate::schema::{OutputPathExt, WorkspacePathExt, model, paths};

use self::project_label::ProjectLabel;
use self::window_tile::WindowTile;

pub struct MainBarInit {
    pub title: &'static str,
}

#[shell_macros::model]
pub struct MainBar {
    #[source(paths::SELECTED_OUTPUT.workspaces())]
    pub project_labels: Vec<NodeRef<model::Workspace>>,

    #[source(paths::SELECTED_WORKSPACE.windows())]
    pub window_tiles: Vec<NodeRef<model::Window>>,
}

#[shell_macros::component(model = MainBar)]
#[relm4::component(pub)]
impl SimpleComponent for MainBar {
    type Init = MainBarInit;
    type Input = sources::Msg;
    type Output = ();

    view! {
        gtk::Window {
            gtk::CenterBox {
                set_widget_name: "rsynapse-bar",
                add_css_class: "bar",
                set_orientation: gtk::Orientation::Horizontal,

                #[wrap(Some)]
                set_start_widget = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,

                    #[bind_list(project_labels, row = ProjectLabel)]
                    project_labels -> gtk::Box {
                        set_widget_name: "project-labels",
                        add_css_class: "projects-widget",
                        add_css_class: "workspaces-widget",
                        add_css_class: "projects-list",
                        add_css_class: "workspaces-list",
                        set_halign: gtk::Align::Center,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 4,
                    }
                },

                #[wrap(Some)]
                set_center_widget = &gtk::Box {
                    set_halign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Horizontal,

                    #[bind_list(window_tiles, row = WindowTile)]
                    window_tiles -> gtk::Box {
                        set_widget_name: "workspace-window-list",
                        add_css_class: "workspace-window-list",
                        set_halign: gtk::Align::Center,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 4,
                        set_valign: gtk::Align::Fill,
                        set_vexpand: true,
                    }
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

        let model = MainBar::default();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

fn bar_window_config() -> WindowConfig {
    WindowConfig::new(Layer::Top)
        .with_anchors(
            Anchors::NONE
                .with_edge(Edge::Bottom)
                .with_edge(Edge::Right)
                .with_edge(Edge::Left),
        )
        .with_auto_exclusive_zone()
        .with_namespace("rsynapse-bar")
}
