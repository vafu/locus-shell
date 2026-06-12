use common_providers::upower::{DISPLAY_DEVICE, DisplayDevice};
use locus_provider::{model, paths};
use providers::ProviderExt;
use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    window::{self, Anchors, Edge, Layer, WindowConfig},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

type WindowNode = locus_provider::NodeRef<model::Window>;

pub struct BarInit {
    pub title: &'static str,
}

#[shell_macros::model]
pub struct BarSources {
    #[source(paths::SELECTED_WORKSPACE.windows())]
    pub window_nodes: Vec<String>,

    #[source(battery_percent_source())]
    pub battery_percent: f64,

    #[source(battery_percent_source().map(battery_label))]
    pub battery_label: String,
}

pub struct Bar {
    sources: BarSources,
    windows: WindowRows,
}

#[shell_macros::component(model = BarSources, state = sources)]
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

                #[local_ref]
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
                },

                gtk::Label {
                    set_widget_name: "battery-label",
                    add_css_class: "dev-panel__battery-label",

                    #[bind(battery_label)]
                    set_label: |label| label.as_str(),
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
        let window_list_value = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let window_list = &window_list_value;

        let model = Bar {
            sources: BarSources::default(),
            windows: WindowRows::new(window_list_value.clone()),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        self.sources.update(msg);
        if self.sources.changed(sources::Field::WindowNodes) {
            self.windows.reconcile(&self.sources.window_nodes);
        }
    }
}

#[derive(Debug)]
struct WindowTitle {
    sources: WindowTitleSources,
}

#[derive(Debug)]
struct WindowTitleInit {
    window: WindowNode,
}

#[derive(Debug)]
#[shell_macros::model(module = window_title_sources)]
pub struct WindowTitleSources {
    pub window: WindowNode,

    #[source(window.title())]
    pub title: String,

    #[source(window.is_selected())]
    pub selected: bool,
}

#[shell_macros::component(
    module = window_title_sources,
    model = WindowTitleSources,
    state = sources
)]
#[relm4::component]
impl SimpleComponent for WindowTitle {
    type Init = WindowTitleInit;
    type Input = window_title_sources::Msg;
    type Output = ();

    view! {
        gtk::Label {
            set_ellipsize: gtk::pango::EllipsizeMode::End,
            set_xalign: 0.0,

            #[bind(title)]
            set_label: |title| title.as_str(),

            #[bind(selected)]
            set_css_classes: |selected| window_title_classes(*selected),
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let sources = WindowTitleSources::new(init.window);
        let model = WindowTitle { sources };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

#[derive(Debug)]
struct WindowRows {
    container: gtk::Box,
    rows: BTreeMap<String, Controller<WindowTitle>>,
    order: Vec<String>,
}

impl WindowRows {
    fn new(container: gtk::Box) -> Self {
        Self {
            container,
            rows: BTreeMap::new(),
            order: Vec::new(),
        }
    }

    fn reconcile(&mut self, nodes: &[String]) {
        let visible: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
        let stale: Vec<String> = self
            .rows
            .keys()
            .filter(|node| !visible.contains(node.as_str()))
            .cloned()
            .collect();

        for node in stale {
            if let Some(row) = self.rows.remove(&node) {
                self.container.remove(row.widget());
            }
        }

        for node in nodes {
            if !self.rows.contains_key(node) {
                let row = WindowTitle::builder()
                    .launch(WindowTitleInit {
                        window: locus_provider::node::<model::Window>(node.clone()),
                    })
                    .detach();
                self.rows.insert(node.clone(), row);
            }
        }

        if self.order != nodes {
            self.render_order(nodes);
        }
        self.order = nodes.to_vec();
    }

    fn render_order(&self, nodes: &[String]) {
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        for node in nodes {
            if let Some(row) = self.rows.get(node) {
                self.container.append(row.widget());
            }
        }
    }
}

const WINDOW_ROW_CLASSES: &[&str] = &["dev-panel__workspace-window"];
const WINDOW_ROW_SELECTED_CLASSES: &[&str] = &[
    "dev-panel__workspace-window",
    "dev-panel__workspace-window--selected",
];

fn window_title_classes(selected: bool) -> &'static [&'static str] {
    if selected {
        WINDOW_ROW_SELECTED_CLASSES
    } else {
        WINDOW_ROW_CLASSES
    }
}

fn battery_fraction(percent: &f64) -> f64 {
    (percent / 100.0).clamp(0.0, 1.0)
}

type BatteryPercentProvider = providers::SharedProvider<dbus_provider::PropertyBinding<f64>, f64>;

fn battery_percent_source() -> BatteryPercentProvider {
    static SOURCE: OnceLock<BatteryPercentProvider> = OnceLock::new();

    SOURCE
        .get_or_init(|| DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE).shared())
        .clone()
}

fn battery_label(percent: f64) -> String {
    format!("{percent:.0}%")
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
    use super::{WINDOW_ROW_CLASSES, WINDOW_ROW_SELECTED_CLASSES, window_title_classes};

    use std::{
        convert::Infallible,
        sync::{Arc, Mutex},
    };

    use locus_provider::paths;
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

    #[test]
    fn selected_workspace_windows_are_semantic_provider() {
        fn assert_provider<T: Send + 'static, P: Provider<T>>(_provider: P) {}

        let provider = paths::SELECTED_WORKSPACE.windows();

        assert_provider::<Vec<String>, _>(provider);
    }

    #[test]
    fn window_title_sources_are_local_to_window_node() {
        fn assert_provider<T: Send + 'static, P: Provider<T>>(_provider: P) {}

        let window = locus_provider::node::<locus_provider::model::Window>("window:1");

        assert_provider::<String, _>(window.title());
        assert_provider::<bool, _>(window.is_selected());
    }

    #[test]
    fn window_title_classes_track_selection() {
        assert_eq!(window_title_classes(false), WINDOW_ROW_CLASSES);
        assert_eq!(window_title_classes(true), WINDOW_ROW_SELECTED_CLASSES);
    }

    fn battery_level(percent: f64) -> &'static str {
        if percent <= 20.0 { "low" } else { "normal" }
    }
}
