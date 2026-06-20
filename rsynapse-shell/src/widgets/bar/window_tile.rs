use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use crate::sources::{WindowNode, WindowTileKind, WindowTileVm, window_tile_vm};
use crate::widgets::material_icon;

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
            set_css_classes: window_tile_classes(&model.vm),

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
                    set_visible: is_plain_visible(&model.vm),

                    #[watch]
                    set_icon_name: plain_icon_name(&model.vm).as_deref(),
                },

                gtk::Box {
                    add_css_class: "agent-inner",

                    #[watch]
                    set_visible: is_agent_visible(&model.vm),

                    #[local_ref]
                    agent_icon -> gtk::Image {
                        #[watch]
                        set_icon_name: Some(material_icon::icon_name(vm_icon(&model.vm)).as_str()),
                    },

                    gtk::ProgressBar {
                        #[watch]
                        set_css_classes: context_indicator_classes(context_pct(&model.vm)),

                        set_orientation: gtk::Orientation::Vertical,
                        set_inverted: true,
                        set_width_request: 8,
                        set_height_request: 24,
                        set_valign: gtk::Align::Center,

                        #[watch]
                        set_fraction: context_pct(&model.vm) as f64 / 100.0,
                    },

                    gtk::Label {
                        add_css_class: "agent-subagent-badge",

                        #[watch]
                        set_label: agent_badge_label(substatus_count(&model.vm)).as_str(),

                        #[watch]
                        set_visible: substatus_count(&model.vm) > 0,
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
        let agent_icon = material_icon::image(vm_icon(&model.vm));
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

fn window_tile_classes(vm: &Option<WindowTileVm>) -> &'static [&'static str] {
    let Some(vm) = vm else {
        return PLAIN_CLASSES;
    };

    match (vm.kind, vm.active, vm.urgent) {
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

fn is_plain_visible(vm: &Option<WindowTileVm>) -> bool {
    vm.as_ref()
        .is_some_and(|vm| vm.kind != WindowTileKind::Agent)
}

fn is_agent_visible(vm: &Option<WindowTileVm>) -> bool {
    vm.as_ref()
        .is_some_and(|vm| vm.kind == WindowTileKind::Agent)
}

fn vm_icon(vm: &Option<WindowTileVm>) -> &str {
    vm.as_ref().map_or("", |vm| vm.icon.as_str())
}

fn context_pct(vm: &Option<WindowTileVm>) -> u32 {
    vm.as_ref().map_or(0, |vm| vm.context_pct)
}

fn substatus_count(vm: &Option<WindowTileVm>) -> u32 {
    vm.as_ref().map_or(0, |vm| vm.substatus_count)
}

const CONTEXT_NORMAL_CLASSES: &[&str] = &["agent-context-indicator", "normal"];
const CONTEXT_WARN_CLASSES: &[&str] = &["agent-context-indicator", "warn"];
const CONTEXT_HIGH_CLASSES: &[&str] = &["agent-context-indicator", "high"];
const CONTEXT_DANGER_CLASSES: &[&str] = &["agent-context-indicator", "danger"];
const CONTEXT_CRITICAL_CLASSES: &[&str] = &["agent-context-indicator", "critical"];

fn context_indicator_classes(context_pct: u32) -> &'static [&'static str] {
    match context_pct {
        95.. => CONTEXT_CRITICAL_CLASSES,
        90.. => CONTEXT_DANGER_CLASSES,
        75.. => CONTEXT_HIGH_CLASSES,
        50.. => CONTEXT_WARN_CLASSES,
        _ => CONTEXT_NORMAL_CLASSES,
    }
}

fn plain_icon_name(vm: &Option<WindowTileVm>) -> Option<String> {
    let vm = vm.as_ref()?;
    if vm.kind == WindowTileKind::Agent {
        return None;
    }
    if vm.icon.is_empty() {
        Some("application-x-executable-symbolic".to_owned())
    } else {
        Some(vm.icon.clone())
    }
}

fn agent_badge_label(substatus_count: u32) -> String {
    if substatus_count > 9 {
        "9+".to_owned()
    } else {
        substatus_count.to_string()
    }
}
