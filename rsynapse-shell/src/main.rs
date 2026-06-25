mod desktop_icon;
mod theme;
mod widgets;

use shell_core::{ShellApp, css::CssPriority};
use tracing_subscriber::EnvFilter;

use widgets::{MainBar, MainBarInit};

fn main() {
    init_tracing();

    let _ = relm4::RELM_THREADS.set(4);

    ShellApp::new("io.github.Locus.RsynapseShell")
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
        .run_async::<MainBar>(MainBarInit {
            title: "Rsynapse Shell",
        });
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();
}
