mod desktop_icon;
mod hints;
mod locusfs_paths;
pub mod request;
mod theme;
pub mod widgets;

use shell_core::{ShellApp, css::CssPriority};
use tracing_subscriber::EnvFilter;

/// Initialize compact tracing from `RUST_LOG`.
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();
}

/// Build a rsynapse shell application with shared styling, theme, and Relm setup.
pub fn rsynapse_app(app_id: &str) -> ShellApp {
    ShellApp::new(app_id)
        .with_relm_threads(4)
        .with_scss_at_priority(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/stylesheets/rsynapse-shell.scss"
            ),
            CssPriority::User,
        )
        .watch_stylesheets(true)
        .on_startup(|_| {
            adw::init().expect("failed to initialize libadwaita");
            theme::prepare_theme();
        })
}
