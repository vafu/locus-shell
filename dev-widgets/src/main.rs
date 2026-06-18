mod locus;
mod primitives;

use shell_core::ShellApp;

use primitives::{Bar, BarInit};

fn main() {
    ShellApp::new("io.github.Locus.DevWidgets")
        .with_scss(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/stylesheets/dev-widgets.scss"
        ))
        .watch_stylesheets(true)
        .run::<Bar>(BarInit {
            title: "Locus Dev Widgets",
        });
}
