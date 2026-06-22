mod agent;
mod source;

use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use self::source::{AgentVisualState, WindowTileKind, WindowTileVm, window_tile_vm};
use super::WindowNode;
use crate::widgets::{
    level_indicator::{self, LevelRenderStyle, LevelStage, LineStyle},
    material_icon,
};

const CONTEXT_STYLE: LevelRenderStyle = LevelRenderStyle::Line(LineStyle::vertical(3.0));
const CONTEXT_STAGES: &[LevelStage] = &[
    LevelStage {
        level: 0.0,
        class: "normal",
    },
    LevelStage {
        level: 50.0,
        class: "warn",
    },
    LevelStage {
        level: 75.0,
        class: "high",
    },
    LevelStage {
        level: 90.0,
        class: "danger",
    },
    LevelStage {
        level: 95.0,
        class: "critical",
    },
];

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
                    set_visible: is_plain_visible(&model.vm),

                    #[watch]
                    set_icon_name: plain_icon_name(&model.vm).as_deref(),
                },

                gtk::Box {
                    add_css_class: "agent-inner",
                    set_valign: gtk::Align::Fill,
                    set_vexpand: true,

                    #[watch]
                    set_visible: is_agent_visible(&model.vm),

                    #[local_ref]
                    agent_icon -> gtk::Image {
                        #[watch]
                        set_icon_name: Some(material_icon::icon_name(vm_icon(&model.vm)).as_str()),
                    },

                    gtk::Overlay {
                        #[watch]
                        set_css_classes: &context_indicator_root_classes(),
                        set_valign: gtk::Align::Fill,
                        set_vexpand: true,

                        add_overlay = &gtk::DrawingArea {
                            set_css_classes: level_indicator::TRACK_CLASSES,
                            set_content_width: 8,
                            set_vexpand: true,
                            set_valign: gtk::Align::Fill,
                            set_draw_func: level_indicator::track_draw_func(CONTEXT_STYLE),
                        },

                        add_overlay = &gtk::DrawingArea {
                            #[watch]
                            set_css_classes: &context_indicator_level_classes(context_pct(&model.vm)),
                            set_content_width: 8,
                            set_vexpand: true,
                            set_valign: gtk::Align::Fill,
                            #[watch]
                            set_draw_func: level_indicator::level_draw_func(
                                f64::from(context_pct(&model.vm)),
                                0.0,
                                100.0,
                                CONTEXT_STYLE,
                            ),
                        }
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
        WindowTileKind::Agent => "workspace-window-agent",
        WindowTileKind::Neovim => "workspace-window-neovim",
    });

    if vm.kind == WindowTileKind::Agent {
        classes.push("agent-window");
    }
    if vm.active {
        classes.push("active");
    }
    if vm.urgent {
        classes.push("urgent");
    }
    if vm.attention {
        classes.push("attention");
    }
    match vm.agent_state {
        AgentVisualState::None => {}
        AgentVisualState::Thinking => classes.push("thinking"),
        AgentVisualState::ToolUse => classes.push("tool-use"),
        AgentVisualState::Compacting => classes.push("compacting"),
    }

    classes
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

fn context_indicator_root_classes() -> Vec<&'static str> {
    level_indicator::root_classes(["line", "agent-context-indicator"])
}

fn context_indicator_level_classes(context_pct: u32) -> Vec<&'static str> {
    level_indicator::level_classes(f64::from(context_pct), 0.0, CONTEXT_STAGES)
}

fn agent_badge_label(count: u32) -> String {
    count.to_string()
}

fn plain_icon_name(vm: &Option<WindowTileVm>) -> Option<String> {
    let vm = vm.as_ref()?;
    if vm.kind == WindowTileKind::Agent {
        None
    } else {
        Some(vm.icon.clone())
    }
}
