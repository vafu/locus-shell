mod providers;
mod schema;
mod theme;
mod widgets;

use shell_core::ShellApp;

use widgets::{MainBar, MainBarInit};

fn main() {
    ShellApp::new("io.github.Locus.RsynapseShell")
        .with_scss(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/stylesheets/rsynapse-shell.scss"
        ))
        .watch_stylesheets(true)
        .on_startup(|_| theme::prepare_theme())
        .run::<MainBar>(MainBarInit {
            title: "Rsynapse Shell",
        });
}
