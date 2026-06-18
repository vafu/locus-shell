mod project_label;
mod window_tile;

use std::{process::Command, thread};

use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    list::ComponentListBoxExt,
    window::{self, Anchors, Edge, Layer, WindowConfig},
};

use crate::sources::{AudioView, BatteryView, ClockView, NetworkView, WindowNode, WorkspaceNode};

use self::project_label::ProjectLabel;
use self::window_tile::WindowTile;

pub struct MainBarInit {
    pub title: &'static str,
}

#[shell_macros::model]
pub struct MainBar {
    #[source(crate::sources::workspaces())]
    pub project_labels: Vec<WorkspaceNode>,

    #[source(crate::sources::selected_workspace_windows())]
    pub window_tiles: Vec<WindowNode>,

    #[source(crate::sources::battery_status())]
    pub battery: BatteryView,

    #[source(crate::sources::network_status())]
    pub network: NetworkView,

    #[source(crate::sources::audio_status())]
    pub audio: AudioView,

    #[source(crate::sources::clock())]
    pub clock: ClockView,
}

#[shell_macros::component(model = MainBar)]
#[relm4::component(pub)]
impl SimpleComponent for MainBar {
    type Init = MainBarInit;
    type Input = sources::Msg;
    type Output = ();

    view! {
        gtk::Window {
            add_css_class: "bar-window",

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
                },

                #[wrap(Some)]
                set_end_widget = &gtk::Box {
                    add_css_class: "system-cluster",
                    set_halign: gtk::Align::End,
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::Box {
                        add_css_class: "barblock",
                        set_halign: gtk::Align::End,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 0,

                        gtk::Image {
                            add_css_class: "panel-widget",
                            add_css_class: "network-icon",
                            add_css_class: "ethernet-icon",
                            #[watch]
                            set_visible: model.network.ethernet.visible,
                            #[watch]
                            set_tooltip_text: Some(model.network.ethernet.tooltip.as_str()),
                            #[watch]
                            set_icon_name: Some(model.network.ethernet.icon.as_str()),
                        },

                        gtk::Image {
                            add_css_class: "panel-widget",
                            add_css_class: "network-icon",
                            add_css_class: "wifi-icon",
                            #[watch]
                            set_visible: model.network.wifi.visible,
                            #[watch]
                            set_tooltip_text: Some(model.network.wifi.tooltip.as_str()),
                            #[watch]
                            set_icon_name: Some(model.network.wifi.icon.as_str()),
                        },

                        gtk::Image {
                            add_css_class: "panel-widget",
                            add_css_class: "audio-icon",
                            #[watch]
                            set_visible: model.audio.visible,
                            #[watch]
                            set_tooltip_text: Some(model.audio.tooltip.as_str()),
                            #[watch]
                            set_icon_name: Some(model.audio.icon.as_str()),
                        },

                        gtk::Image {
                            add_css_class: "panel-widget",
                            add_css_class: "battery-icon",
                            #[watch]
                            set_visible: model.battery.present,
                            #[watch]
                            set_tooltip_text: Some(battery_tooltip(&model.battery).as_str()),
                            #[watch]
                            set_icon_name: Some(battery_icon_name(&model.battery).as_str()),
                        }
                    },

                    #[name = "clock_button"]
                    gtk::Button {
                        add_css_class: "barblock",
                        add_css_class: "panel-button",
                        add_css_class: "flat",
                        add_css_class: "circular",
                        add_css_class: "clock-widget",
                        #[watch]
                        set_tooltip_text: Some(model.clock.date.as_str()),

                        gtk::Label {
                            add_css_class: "clock-label",
                            #[watch]
                            set_label: model.clock.time.as_str(),
                        }
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
        widgets.clock_button.connect_clicked(|_| {
            thread::spawn(|| {
                let _ = Command::new("swaync-client").arg("-t").status();
            });
        });

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

fn battery_tooltip(battery: &BatteryView) -> String {
    format!("{}%", battery.percent)
}

fn battery_icon_name(battery: &BatteryView) -> String {
    if !battery.present {
        return "battery-missing-symbolic".to_owned();
    }

    let charging = battery.state.is_charging();

    if battery.percent >= 95 && charging {
        return "battery-level-100-charged-symbolic".to_owned();
    }

    let level = (((battery.percent.min(100) as u16) + 5) / 10) * 10;
    let state = if charging { "-charging" } else { "" };

    format!("battery-level-{level}{state}-symbolic")
}
