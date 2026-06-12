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

pub struct BarInit {
    pub title: &'static str,
}

#[shell_macros::model]
pub struct BarSources {
    #[source(paths::SELECTED_WINDOW.target())]
    pub selected_window_id: String,

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
    window_error: Option<WindowWatchError>,
}

#[derive(Debug)]
pub enum BarMsg {
    Sources(sources::Msg),
    WindowTitleChanged { node: String, title: String },
    WindowTitleWatchFailed { node: String, error: String },
}

impl From<sources::Msg> for BarMsg {
    fn from(msg: sources::Msg) -> Self {
        Self::Sources(msg)
    }
}

#[shell_macros::component(model = BarSources, state = sources)]
#[relm4::component(pub)]
impl SimpleComponent for Bar {
    type Init = BarInit;
    type Input = BarMsg;
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
            window_error: None,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BarMsg::Sources(msg) => {
                self.sources.update(msg);
                if self.sources.changed(sources::Field::WindowNodes) {
                    self.windows.reconcile(
                        &self.sources.window_nodes,
                        &self.sources.selected_window_id,
                        &sender,
                    );
                } else if self.sources.changed(sources::Field::SelectedWindowId) {
                    self.windows
                        .set_selected_window(&self.sources.selected_window_id);
                }
            }
            BarMsg::WindowTitleChanged { node, title } => {
                self.windows.set_title(&node, title);
            }
            BarMsg::WindowTitleWatchFailed { node, error } => {
                self.window_error = Some(WindowWatchError { node, error });
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct WindowWatchError {
    node: String,
    error: String,
}

#[derive(Debug)]
struct WindowRows {
    container: gtk::Box,
    rows: BTreeMap<String, WindowRow>,
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

    fn reconcile(
        &mut self,
        nodes: &[String],
        selected_window: &str,
        sender: &ComponentSender<Bar>,
    ) {
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
                self.rows
                    .insert(node.clone(), WindowRow::new(node.clone(), sender));
            }
        }

        if self.order != nodes {
            self.render_order(nodes);
        }
        self.order = nodes.to_vec();
        self.set_selected_window(selected_window);
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

    fn set_selected_window(&mut self, selected_window: &str) {
        for (node, row) in &mut self.rows {
            row.set_selected(node == selected_window);
        }
    }

    fn set_title(&mut self, node: &str, title: String) {
        if let Some(row) = self.rows.get_mut(node) {
            row.set_title(title);
        }
    }
}

#[derive(Debug)]
struct WindowRow {
    node: String,
    label: gtk::Label,
    title: String,
    selected: bool,
    _subscription: providers::Subscription,
}

impl WindowRow {
    fn new(node: String, sender: &ComponentSender<Bar>) -> Self {
        let label = gtk::Label::new(None);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_xalign(0.0);
        label.set_css_classes(WINDOW_ROW_CLASSES);
        label.set_label(window_title_text(&node, ""));

        Self {
            node: node.clone(),
            label,
            title: String::new(),
            selected: false,
            _subscription: start_window_title_subscription(node, sender),
        }
    }

    fn widget(&self) -> &gtk::Label {
        &self.label
    }

    fn set_title(&mut self, title: String) {
        self.title = title;
        self.label
            .set_label(window_title_text(&self.node, &self.title));
        self.apply_css_classes();
    }

    fn set_selected(&mut self, selected: bool) {
        if selected != self.selected {
            self.selected = selected;
            self.apply_css_classes();
        }
    }

    fn apply_css_classes(&self) {
        self.label
            .set_css_classes(window_row_classes(self.selected, self.title.is_empty()));
    }
}

const WINDOW_ROW_CLASSES: &[&str] = &["dev-panel__workspace-window"];
const WINDOW_ROW_EMPTY_CLASSES: &[&str] = &[
    "dev-panel__workspace-window",
    "dev-panel__workspace-window--empty",
];
const WINDOW_ROW_SELECTED_CLASSES: &[&str] = &[
    "dev-panel__workspace-window",
    "dev-panel__workspace-window--selected",
];
const WINDOW_ROW_SELECTED_EMPTY_CLASSES: &[&str] = &[
    "dev-panel__workspace-window",
    "dev-panel__workspace-window--selected",
    "dev-panel__workspace-window--empty",
];

fn window_row_classes(selected: bool, empty: bool) -> &'static [&'static str] {
    match (selected, empty) {
        (true, true) => WINDOW_ROW_SELECTED_EMPTY_CLASSES,
        (true, false) => WINDOW_ROW_SELECTED_CLASSES,
        (false, true) => WINDOW_ROW_EMPTY_CLASSES,
        (false, false) => WINDOW_ROW_CLASSES,
    }
}

fn window_title_text<'node>(node: &'node str, title: &'node str) -> &'node str {
    if title.is_empty() { node } else { title }
}

fn start_window_title_subscription(
    node: String,
    sender: &ComponentSender<Bar>,
) -> providers::Subscription {
    let mut subscription = providers::Subscription::new();
    let context = subscription.context();
    let update_sender = sender.clone();
    let error_sender = sender.clone();
    let node_for_provider = node.clone();
    let node_for_update = node.clone();
    let task = providers::spawn(async move {
        let source =
            locus_provider::node::<model::Window>(node_for_provider).property(model::Window::TITLE);
        let result = providers::run_provider(source, context, move |title| {
            update_sender.input(BarMsg::WindowTitleChanged {
                node: node_for_update.clone(),
                title,
            });
        })
        .await;

        if let Err(error) = result {
            error_sender.input(BarMsg::WindowTitleWatchFailed {
                node,
                error: error.to_string(),
            });
        }
    });
    subscription.set_task(task);
    subscription
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
    use super::{
        WINDOW_ROW_CLASSES, WINDOW_ROW_EMPTY_CLASSES, WINDOW_ROW_SELECTED_CLASSES,
        WINDOW_ROW_SELECTED_EMPTY_CLASSES, window_row_classes, window_title_text,
    };

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
    fn window_title_falls_back_to_node_id() {
        assert_eq!(window_title_text("window:1", ""), "window:1");
        assert_eq!(window_title_text("window:1", "Terminal"), "Terminal");
    }

    #[test]
    fn window_row_classes_track_selection_and_empty_title() {
        assert_eq!(window_row_classes(false, false), WINDOW_ROW_CLASSES);
        assert_eq!(window_row_classes(true, false), WINDOW_ROW_SELECTED_CLASSES);
        assert_eq!(window_row_classes(false, true), WINDOW_ROW_EMPTY_CLASSES);
        assert_eq!(
            window_row_classes(true, true),
            WINDOW_ROW_SELECTED_EMPTY_CLASSES
        );
    }

    fn battery_level(percent: f64) -> &'static str {
        if percent <= 20.0 { "low" } else { "normal" }
    }
}
