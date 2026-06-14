use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use locus_provider::NodeRef;

use crate::providers::{WindowTileKind, WindowTileView, window_tile_for_window};
use crate::schema::{WindowNodeExt, model};
use crate::widgets::material_icon;

#[derive(Debug)]
#[shell_macros::model(module = window_tile_sources)]
pub(super) struct WindowTile {
    pub window: NodeRef<model::Window>,

    #[source(window_tile_for_window(window.id().to_owned()))]
    pub view: WindowTileView,

    #[source(window.is_selected())]
    pub active: bool,
}

#[shell_macros::component(
    module = window_tile_sources,
    model = WindowTile
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for WindowTile {
    type Init = NodeRef<model::Window>;
    type Input = window_tile_sources::Msg;
    type Output = ();

    view! {
        gtk::Box {
            #[watch]
            set_css_classes: window_tile_classes(model.view.kind, model.active, model.view.urgent),

            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Fill,
            set_vexpand: true,

            #[watch]
            set_tooltip_text: Some(model.view.tooltip.as_str()),

            gtk::Box {
                add_css_class: "workspace-window-content",
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Fill,
                set_vexpand: true,

                gtk::Image {
                    #[watch]
                    set_visible: model.view.kind != WindowTileKind::Agent,

                    #[watch]
                    set_icon_name: Some(plain_icon_name(&model.view.icon).as_str()),
                },

                gtk::Box {
                    add_css_class: "agent-inner",

                    #[watch]
                    set_visible: model.view.kind == WindowTileKind::Agent,

                    #[local_ref]
                    agent_icon -> gtk::Image {
                        #[watch]
                        set_icon_name: Some(material_icon::icon_name(&model.view.icon).as_str()),
                    },

                    gtk::ProgressBar {
                        add_css_class: "levelindicator",
                        add_css_class: "line",

                        #[watch]
                        set_fraction: model.view.context_pct as f64 / 100.0,
                    },

                    gtk::Label {
                        add_css_class: "agent-subagent-badge",

                        #[watch]
                        set_label: agent_badge_label(model.view.substatus_count).as_str(),

                        #[watch]
                        set_visible: model.view.substatus_count > 0,
                    }
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
        let agent_icon = material_icon::image(&model.view.icon);
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

const PLAIN_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-plain",
];
const PLAIN_ACTIVE_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-plain",
    "active",
];
const PLAIN_URGENT_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-plain",
    "urgent",
];
const PLAIN_ACTIVE_URGENT_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-plain",
    "active",
    "urgent",
];
const AGENT_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-agent",
    "agent-window",
];
const AGENT_ACTIVE_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-agent",
    "agent-window",
    "active",
];
const AGENT_URGENT_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-agent",
    "agent-window",
    "urgent",
];
const AGENT_ACTIVE_URGENT_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-agent",
    "agent-window",
    "active",
    "urgent",
];
const NEOVIM_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-neovim",
];
const NEOVIM_ACTIVE_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-neovim",
    "active",
];
const NEOVIM_URGENT_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-neovim",
    "urgent",
];
const NEOVIM_ACTIVE_URGENT_CLASSES: &[&str] = &[
    "workspace-window-frame",
    "workspace-window-tile",
    "workspace-window-neovim",
    "active",
    "urgent",
];

fn window_tile_classes(
    kind: WindowTileKind,
    active: bool,
    urgent: bool,
) -> &'static [&'static str] {
    match (kind, active, urgent) {
        (WindowTileKind::Agent, true, true) => AGENT_ACTIVE_URGENT_CLASSES,
        (WindowTileKind::Agent, true, false) => AGENT_ACTIVE_CLASSES,
        (WindowTileKind::Agent, false, true) => AGENT_URGENT_CLASSES,
        (WindowTileKind::Agent, false, false) => AGENT_CLASSES,
        (WindowTileKind::Neovim, true, true) => NEOVIM_ACTIVE_URGENT_CLASSES,
        (WindowTileKind::Neovim, true, false) => NEOVIM_ACTIVE_CLASSES,
        (WindowTileKind::Neovim, false, true) => NEOVIM_URGENT_CLASSES,
        (WindowTileKind::Neovim, false, false) => NEOVIM_CLASSES,
        (WindowTileKind::Plain, true, true) => PLAIN_ACTIVE_URGENT_CLASSES,
        (WindowTileKind::Plain, true, false) => PLAIN_ACTIVE_CLASSES,
        (WindowTileKind::Plain, false, true) => PLAIN_URGENT_CLASSES,
        (WindowTileKind::Plain, false, false) => PLAIN_CLASSES,
    }
}

fn plain_icon_name(icon: &str) -> String {
    if icon.is_empty() {
        "application-x-executable-symbolic".to_owned()
    } else {
        icon.to_owned()
    }
}

fn agent_badge_label(substatus_count: u32) -> String {
    if substatus_count > 9 {
        "9+".to_owned()
    } else {
        substatus_count.to_string()
    }
}
