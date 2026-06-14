use std::fmt::{self, Debug};
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use gtk::prelude::ApplicationExt;
use relm4::{Component, RelmApp};

use crate::css::{
    CssPriority, SassConfig, Stylesheet, StylesheetError, StylesheetSource, StylesheetWatcher,
};
use crate::runtime;

pub struct ShellApp {
    app_id: String,
    stylesheets: Vec<StylesheetRegistration>,
    watch_stylesheets: bool,
    sass_config: SassConfig,
    startup_handlers: Vec<StartupHandler>,
}

impl ShellApp {
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            stylesheets: Vec::new(),
            watch_stylesheets: false,
            sass_config: SassConfig::default(),
            startup_handlers: Vec::new(),
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

    pub fn on_startup(mut self, handler: impl Fn(&gtk::Application) + 'static) -> Self {
        self.startup_handlers.push(Box::new(handler));
        self
    }

    pub fn run<C>(self, payload: C::Init)
    where
        C: Component,
        C::Input: Debug + 'static,
        C::Root: AsRef<gtk::Window>,
    {
        let Self {
            app_id,
            stylesheets,
            watch_stylesheets,
            sass_config,
            startup_handlers,
        } = self;

        runtime::ensure_provider_runtime();

        let app = RelmApp::<C::Input>::new(&app_id);
        let stylesheets = Self::prepare_stylesheets(stylesheets, sass_config)
            .expect("failed to initialize shell app stylesheets");
        let gtk_app = relm4::main_application();
        let stylesheets = Rc::new(RefCell::new(Some(stylesheets)));
        let stylesheet_watchers: Rc<RefCell<Vec<StylesheetWatcher>>> =
            Rc::new(RefCell::new(Vec::new()));
        let startup_handlers = Rc::new(RefCell::new(Some(startup_handlers)));

        gtk_app.connect_startup(move |app| {
            if let Some(startup_handlers) = startup_handlers.borrow_mut().take() {
                for handler in startup_handlers {
                    handler(app);
                }
            }

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

    fn prepare_stylesheets(
        registrations: Vec<StylesheetRegistration>,
        sass_config: SassConfig,
    ) -> Result<Vec<Stylesheet>, StylesheetError> {
        let mut stylesheets = Vec::new();

        for registration in registrations {
            let mut stylesheet = Stylesheet::new(
                registration.source,
                registration.priority,
                sass_config.clone(),
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

type StartupHandler = Box<dyn Fn(&gtk::Application)>;

impl Debug for ShellApp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ShellApp")
            .field("app_id", &self.app_id)
            .field("stylesheets", &self.stylesheets)
            .field("watch_stylesheets", &self.watch_stylesheets)
            .field("sass_config", &self.sass_config)
            .field("startup_handlers", &self.startup_handlers.len())
            .finish()
    }
}
