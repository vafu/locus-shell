mod source;

use std::time::Duration;

use gtk4_background_effect::BackgroundEffectRegion;
use relm4::prelude::*;
use shell_core::{
    gtk::{self, glib, prelude::*},
    window::{self, Anchors, Edge, Layer, WindowConfig},
};

use self::source::{OsdLevel, osd_level};
use crate::widgets::BACKGROUND_BLUR_CLASS;

const OSD_HIDE_DELAY: Duration = Duration::from_secs(1);
const OSD_BACKGROUND_BLUR_CLASSES: &[&str] = &[BACKGROUND_BLUR_CLASS];
const OSD_BACKGROUND_BLUR_RADIUS: i32 = 24;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OsdInit {
    pub title: &'static str,
}

#[derive(Debug)]
#[shell_macros::model(module = osd_sources)]
pub struct OsdWindow {
    #[source(osd_level())]
    level: OsdLevel,
    payload: OsdPayload,
    generation: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
enum OsdPayload {
    #[default]
    Hidden,
    Level {
        value: f64,
        icon_name: String,
    },
}

#[derive(Debug)]
pub enum OsdInput {
    Source(osd_sources::Msg),
    Hide(u64),
}

impl From<osd_sources::Msg> for OsdInput {
    fn from(msg: osd_sources::Msg) -> Self {
        Self::Source(msg)
    }
}

#[shell_macros::component(module = osd_sources, model = OsdWindow)]
#[relm4::component(pub, async)]
impl SimpleAsyncComponent for OsdWindow {
    type Init = OsdInit;
    type Input = OsdInput;
    type Output = ();

    view! {
        gtk::Window {
            add_css_class: "OSD",
            set_visible: true,

            gtk::Revealer {
                #[watch]
                set_reveal_child: !matches!(model.payload, OsdPayload::Hidden),
                set_transition_type: gtk::RevealerTransitionType::Crossfade,

                gtk::Box {
                    add_css_class: "osd-shell",
                    add_css_class: BACKGROUND_BLUR_CLASS,
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Image {
                        #[watch]
                        set_icon_name: osd_icon_name(&model.payload),
                        set_pixel_size: 48,
                    },

                    gtk::LevelBar {
                        set_valign: gtk::Align::Center,
                        set_width_request: 100,
                        set_min_value: 0.0,
                        set_max_value: 1.0,
                        #[watch]
                        set_value: osd_value(&model.payload),
                    }
                }
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        window::apply_layer_shell_config(&root, osd_window_config());
        root.set_title(Some(init.title));

        let model = OsdWindow::new(OsdPayload::Hidden, 0);
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            OsdInput::Source(msg) => {
                OsdWindow::update(self, msg);
                self.generation = self.generation.wrapping_add(1);
                self.payload = OsdPayload::Level {
                    value: self.level.value,
                    icon_name: self.level.icon_name.clone(),
                };

                let generation = self.generation;
                glib::timeout_add_local_once(OSD_HIDE_DELAY, move || {
                    sender.input(OsdInput::Hide(generation));
                });
            }
            OsdInput::Hide(generation) if generation == self.generation => {
                self.payload = OsdPayload::Hidden;
            }
            OsdInput::Hide(_) => {}
        }
    }
}

fn osd_window_config() -> WindowConfig {
    WindowConfig::new(Layer::Overlay)
        .with_anchors(Anchors::NONE.with_edge(Edge::Bottom))
        .with_background_blur_region(BackgroundEffectRegion::RoundedCssClasses {
            classes: OSD_BACKGROUND_BLUR_CLASSES,
            radius: OSD_BACKGROUND_BLUR_RADIUS,
        })
        .with_namespace("OSD")
}

fn osd_icon_name(payload: &OsdPayload) -> Option<&str> {
    match payload {
        OsdPayload::Hidden => None,
        OsdPayload::Level { icon_name, .. } => Some(icon_name.as_str()),
    }
}

fn osd_value(payload: &OsdPayload) -> f64 {
    match payload {
        OsdPayload::Hidden => 0.0,
        OsdPayload::Level { value, .. } => *value,
    }
}
