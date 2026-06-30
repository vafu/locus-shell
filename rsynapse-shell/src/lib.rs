mod desktop_icon;
mod hints;
mod locusfs_paths;
pub mod request;
mod theme;
pub mod widgets;

use std::path::PathBuf;

use shell_core::{ShellApp, css::CssPriority};
use tracing_subscriber::EnvFilter;

const SHELL_STYLESHEET: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/stylesheets/rsynapse-shell.scss"
);
const SHELL_STYLESHEET_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/stylesheets");

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
        .with_sass_load_path(SHELL_STYLESHEET_DIR)
        .with_scss_at_priority(rsynapse_stylesheet(), CssPriority::User)
        .watch_stylesheets(true)
        .on_startup(|_| {
            adw::init().expect("failed to initialize libadwaita");
            theme::prepare_theme();
        })
}

fn rsynapse_stylesheet() -> PathBuf {
    let local = config_home().join("rsynapse/shell.scss");
    if local.exists() {
        local
    } else {
        PathBuf::from(SHELL_STYLESHEET)
    }
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}
