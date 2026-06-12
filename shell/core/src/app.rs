use std::fmt::Debug;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use gtk::prelude::ApplicationExt;
use relm4::{Component, RelmApp};

use crate::css::{
    CssPriority, SassConfig, Stylesheet, StylesheetError, StylesheetSource, StylesheetWatcher,
};
use crate::runtime;

#[derive(Debug)]
pub struct ShellApp {
    app_id: String,
    stylesheets: Vec<StylesheetRegistration>,
    watch_stylesheets: bool,
    sass_config: SassConfig,
}

impl ShellApp {
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            stylesheets: Vec::new(),
            watch_stylesheets: false,
            sass_config: SassConfig::default(),
        }
    }

    pub fn with_stylesheet(mut self, path: impl Into<PathBuf>) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::new(path),
            priority: CssPriority::Application,
        });
        self
    }

    pub fn with_stylesheet_at_priority(
        mut self,
        path: impl Into<PathBuf>,
        priority: CssPriority,
    ) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::new(path),
            priority,
        });
        self
    }

    pub fn with_css(mut self, path: impl Into<PathBuf>) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::new(path),
            priority: CssPriority::Application,
        });
        self
    }

    pub fn with_css_at_priority(mut self, path: impl Into<PathBuf>, priority: CssPriority) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::new(path),
            priority,
        });
        self
    }

    pub fn with_scss(mut self, path: impl Into<PathBuf>) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::new(path),
            priority: CssPriority::Application,
        });
        self
    }

    pub fn with_scss_at_priority(
        mut self,
        path: impl Into<PathBuf>,
        priority: CssPriority,
    ) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::new(path),
            priority,
        });
        self
    }

    pub fn with_sass_load_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.sass_config.add_load_path(path);
        self
    }

    pub const fn watch_stylesheets(mut self, watch_stylesheets: bool) -> Self {
        self.watch_stylesheets = watch_stylesheets;
        self
    }

    pub fn run<C>(self, payload: C::Init)
    where
        C: Component,
        C::Input: Debug + 'static,
        C::Root: AsRef<gtk::Window>,
    {
        runtime::ensure_provider_runtime();

        let app = RelmApp::<C::Input>::new(&self.app_id);
        let watch_stylesheets = self.watch_stylesheets;
        let stylesheets = self
            .prepare_stylesheets()
            .expect("failed to initialize shell app stylesheets");
        let gtk_app = relm4::main_application();
        let stylesheets = Rc::new(RefCell::new(Some(stylesheets)));
        let stylesheet_watchers: Rc<RefCell<Vec<StylesheetWatcher>>> =
            Rc::new(RefCell::new(Vec::new()));

        gtk_app.connect_startup(move |_| {
            let Some(stylesheets) = stylesheets.borrow_mut().take() else {
                return;
            };

            for stylesheet in stylesheets {
                stylesheet.install();

                if watch_stylesheets {
                    let watcher = stylesheet
                        .watch()
                        .expect("failed to initialize shell app stylesheet watcher");
                    stylesheet_watchers.borrow_mut().push(watcher);
                }
            }
        });

        app.run::<C>(payload);
    }

    fn prepare_stylesheets(self) -> Result<Vec<Stylesheet>, StylesheetError> {
        let mut stylesheets = Vec::new();

        for registration in self.stylesheets {
            let mut stylesheet = Stylesheet::new(
                registration.source,
                registration.priority,
                self.sass_config.clone(),
            );
            stylesheet.load()?;
            stylesheets.push(stylesheet);
        }

        Ok(stylesheets)
    }
}

#[derive(Debug)]
struct StylesheetRegistration {
    source: StylesheetSource,
    priority: CssPriority,
}
