use super::{
    CssPriority, StylesheetError, StylesheetSource,
    fingerprint::{StylesheetFingerprint, stylesheet_fingerprint},
};

#[derive(Debug)]
pub struct Stylesheet {
    source: StylesheetSource,
    priority: CssPriority,
    provider: gtk::CssProvider,
    fingerprint: Option<StylesheetFingerprint>,
}

impl Stylesheet {
    pub fn new(source: StylesheetSource, priority: CssPriority) -> Self {
        Self {
            source,
            priority,
            provider: gtk::CssProvider::new(),
            fingerprint: None,
        }
    }

    pub fn load(&mut self) -> Result<(), StylesheetError> {
        let css = self.source.load()?;
        self.provider.load_from_data(&css);
        self.fingerprint = Some(stylesheet_fingerprint(&self.source.watch_root())?);
        Ok(())
    }

    pub fn install(&self) {
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &self.provider,
                self.priority.gtk_priority(),
            );
        }
    }

    pub fn watch(self) {
        let source = self.source;
        let provider = self.provider;
        let mut fingerprint = self.fingerprint;

        gtk::glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
            let watch_root = source.watch_root();
            let next_fingerprint = match stylesheet_fingerprint(&watch_root) {
                Ok(fingerprint) => fingerprint,
                Err(error) => {
                    eprintln!("{error}");
                    return gtk::glib::ControlFlow::Continue;
                }
            };

            if fingerprint.as_ref() == Some(&next_fingerprint) {
                return gtk::glib::ControlFlow::Continue;
            }

            match source.load() {
                Ok(css) => {
                    provider.load_from_data(&css);
                    fingerprint = Some(next_fingerprint);
                }
                Err(error) => {
                    fingerprint = Some(next_fingerprint);
                    eprintln!("{error}");
                }
            }

            gtk::glib::ControlFlow::Continue
        });
    }
}
