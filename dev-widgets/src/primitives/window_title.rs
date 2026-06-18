use relm4::prelude::*;
use shell_core::gtk::prelude::*;

use crate::locus::{WindowNode, window_is_selected, window_title};

#[derive(Debug)]
#[shell_macros::model(module = window_title_sources)]
pub(super) struct WindowTitle {
    pub window: WindowNode,

    #[source(window_title(window.clone()))]
    pub title: String,

    #[source(window_is_selected(window.clone()))]
    pub selected: bool,
}

#[shell_macros::component(
    module = window_title_sources,
    model = WindowTitle
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for WindowTitle {
    type Init = WindowNode;
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
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = WindowTitle::new(init);
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

pub(super) const WINDOW_ROW_CLASSES: &[&str] = &["dev-panel__workspace-window"];
pub(super) const WINDOW_ROW_SELECTED_CLASSES: &[&str] = &[
    "dev-panel__workspace-window",
    "dev-panel__workspace-window--selected",
];

pub(super) fn window_title_classes(selected: bool) -> &'static [&'static str] {
    if selected {
        WINDOW_ROW_SELECTED_CLASSES
    } else {
        WINDOW_ROW_CLASSES
    }
}
