use locus_provider::{model, paths};
use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    window::{self, Anchors, Edge, Layer, WindowConfig},
};
use standard_dbus::upower::{DISPLAY_DEVICE, DisplayDevice};

pub struct BarInit {
    pub title: &'static str,
}

#[shell_macros::model]
pub struct BarLocus {
    #[locus(
        source = paths::SELECTED_WINDOW
            .property(model::Window::TITLE)
    )]
    pub selected_window_title: String,

    #[locus(source = DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE))]
    pub battery_percent: f64,
}

pub struct Bar {
    locus: BarLocus,
}

#[shell_macros::component(model = BarLocus)]
#[relm4::component(pub)]
impl SimpleComponent for Bar {
    type Init = BarInit;
    type Input = locus::Msg;
    type Output = ();

    view! {
        gtk::Window {
            gtk::Box {
                set_widget_name: "dev-bar",
                add_css_class: "dev-panel",
                set_orientation: gtk::Orientation::Horizontal,

                gtk::Label {
                    set_widget_name: "selected-window-title",
                    set_hexpand: true,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_xalign: 0.0,

                    #[locus(selected_window_title)]
                    set_label: |title| title.as_str(),

                    #[locus(selected_window_title)]
                    set_css_classes: window_title_classes,
                },

                gtk::ProgressBar {
                    set_widget_name: "battery-percent",
                    add_css_class: "dev-panel__battery",
                    set_show_text: true,

                    #[locus(battery_percent)]
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

        let model = Bar {
            locus: BarLocus::default(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

const WINDOW_TITLE_CLASSES: &[&str] = &["dev-panel__window-title"];
const EMPTY_WINDOW_TITLE_CLASSES: &[&str] =
    &["dev-panel__window-title", "dev-panel__window-title--empty"];

fn window_title_classes(title: &str) -> &'static [&'static str] {
    if title.is_empty() {
        EMPTY_WINDOW_TITLE_CLASSES
    } else {
        WINDOW_TITLE_CLASSES
    }
}

fn battery_fraction(percent: &f64) -> f64 {
    (percent / 100.0).clamp(0.0, 1.0)
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

#[cfg(test)]
mod test {
    use std::{
        convert::Infallible,
        sync::{Arc, Mutex},
    };

    use providers::{Provider, ProviderContext, ProviderExt, ProviderSender, run_provider};

    #[derive(Debug, PartialEq)]
    struct BarSummary {
        title: String,
        battery_level: &'static str,
    }

    struct ValueProvider<T>(T);

    impl<T> Provider<T> for ValueProvider<T>
    where
        T: Send + 'static,
    {
        type Error = Infallible;

        async fn run(
            self,
            _context: ProviderContext,
            sender: ProviderSender<T>,
        ) -> Result<(), Self::Error> {
            sender.send(self.0);
            Ok(())
        }
    }

    #[test]
    fn combines_sources_into_bar_summary() {
        let values = Arc::new(Mutex::new(Vec::new()));
        let captured = values.clone();
        let provider = ValueProvider("Terminal".to_owned()).combine_latest(
            ValueProvider(17.0),
            |title, percent| BarSummary {
                title: title.clone(),
                battery_level: battery_level(*percent),
            },
        );

        let result = futures::executor::block_on(run_provider(
            provider,
            ProviderContext::default(),
            move |summary| {
                captured.lock().expect("summary lock").push(summary);
            },
        ));

        assert!(result.is_ok());
        assert_eq!(
            *values.lock().expect("summary lock"),
            [BarSummary {
                title: "Terminal".to_owned(),
                battery_level: "low",
            }]
        );
    }

    fn battery_level(percent: f64) -> &'static str {
        if percent <= 20.0 { "low" } else { "normal" }
    }
}
