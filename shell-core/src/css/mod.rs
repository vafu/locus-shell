mod compiler;
mod error;
mod fingerprint;
mod source;
mod stylesheet;

use std::path::Path;

use gtk::prelude::*;

pub use error::StylesheetError;
pub use source::StylesheetSource;
pub use stylesheet::Stylesheet;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CssPriority {
    Application,
    User,
}

impl CssPriority {
    pub const fn gtk_priority(self) -> u32 {
        match self {
            Self::Application => gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            Self::User => gtk::STYLE_PROVIDER_PRIORITY_USER,
        }
    }
}

pub fn load_stylesheet(path: impl AsRef<Path>, priority: CssPriority) {
    let provider = gtk::CssProvider::new();
    provider.load_from_path(path.as_ref());

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &provider, priority.gtk_priority());
    }
}

pub fn add_css_classes(widget: &impl IsA<gtk::Widget>, classes: &[&str]) {
    for class in classes {
        widget.add_css_class(class);
    }
}

#[cfg(test)]
mod test;
