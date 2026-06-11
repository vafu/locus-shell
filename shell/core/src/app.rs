use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use gtk::prelude::ApplicationExt;
use relm4::{Component, RelmApp};

use crate::css::{CssPriority, Stylesheet, StylesheetError, StylesheetSource};

#[derive(Debug)]
pub struct ShellApp<M: Debug + 'static> {
    app_id: String,
    stylesheets: Vec<StylesheetRegistration>,
    watch_stylesheets: bool,
    marker: PhantomData<M>,
}

impl<M: Debug + 'static> ShellApp<M> {
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            stylesheets: Vec::new(),
            watch_stylesheets: false,
            marker: PhantomData,
        }
    }

    pub fn with_css(mut self, path: impl Into<PathBuf>) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::css(path),
            priority: CssPriority::Application,
        });
        self
    }

    pub fn with_css_at_priority(mut self, path: impl Into<PathBuf>, priority: CssPriority) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::css(path),
            priority,
        });
        self
    }

    pub fn with_scss(mut self, path: impl Into<PathBuf>) -> Self {
        self.stylesheets.push(StylesheetRegistration {
            source: StylesheetSource::scss(path),
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
            source: StylesheetSource::scss(path),
            priority,
        });
        self
    }

    pub const fn watch_stylesheets(mut self, watch_stylesheets: bool) -> Self {
        self.watch_stylesheets = watch_stylesheets;
        self
    }

    pub fn run<C>(self, payload: C::Init)
    where
        C: Component<Input = M>,
        C::Root: AsRef<gtk::Window>,
    {
        self.try_run::<C>(payload)
            .expect("failed to initialize shell app");
    }

    pub fn try_run<C>(self, payload: C::Init) -> Result<(), StylesheetError>
    where
        C: Component<Input = M>,
        C::Root: AsRef<gtk::Window>,
    {
        let app = RelmApp::new(&self.app_id);
        let watch_stylesheets = self.watch_stylesheets;
        let stylesheets = self.prepare_stylesheets()?;
        let gtk_app = relm4::main_application();
        let stylesheets = Rc::new(RefCell::new(Some(stylesheets)));

        gtk_app.connect_startup(move |_| {
            let Some(stylesheets) = stylesheets.borrow_mut().take() else {
                return;
            };

            for stylesheet in stylesheets {
                stylesheet.install();

                if watch_stylesheets {
                    stylesheet.watch();
                }
            }
        });

        app.run::<C>(payload);
        Ok(())
    }

    fn prepare_stylesheets(self) -> Result<Vec<Stylesheet>, StylesheetError> {
        let mut stylesheets = Vec::new();

        for registration in self.stylesheets {
            let mut stylesheet = Stylesheet::new(registration.source, registration.priority);
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
