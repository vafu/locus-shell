use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque};
use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use crate::locus_schema::model;

use super::window_title::{WindowTitle, WindowTitleInit};

#[derive(Debug)]
struct WindowTitleRow {
    node: String,
    title: Controller<WindowTitle>,
}

#[relm4::factory]
impl FactoryComponent for WindowTitleRow {
    type Init = String;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        root = gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            #[local_ref]
            title -> gtk::Label {}
        }
    }

    fn init_model(node: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let title = WindowTitle::builder()
            .launch(WindowTitleInit {
                window: locus_provider::node::<model::Window>(node.clone()),
            })
            .detach();

        Self { node, title }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let title = self.title.widget();
        let widgets = view_output!();
        widgets
    }
}

#[derive(Debug)]
pub(super) struct WindowRows {
    rows: FactoryVecDeque<WindowTitleRow>,
}

impl WindowRows {
    pub(super) fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let rows = FactoryVecDeque::builder().launch(container).detach();

        Self { rows }
    }

    pub(super) fn widget(&self) -> &gtk::Box {
        self.rows.widget()
    }

    pub(super) fn reconcile(&mut self, nodes: &[String]) {
        let mut rows = self.rows.guard();

        let mut index = 0;
        while index < rows.len() {
            if nodes.iter().any(|node| node == &rows[index].node) {
                index += 1;
            } else {
                rows.remove(index);
            }
        }

        for (target_index, node) in nodes.iter().enumerate() {
            let current_index = { rows.iter().position(|row| row.node == *node) };
            match current_index {
                Some(current_index) if current_index != target_index => {
                    rows.move_to(current_index, target_index);
                }
                Some(_) => {}
                None => {
                    rows.insert(target_index, node.clone());
                }
            }
        }
    }
}
