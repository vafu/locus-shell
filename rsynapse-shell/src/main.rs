mod desktop_icon;
mod theme;
mod widgets;

use shell_core::ShellApp;

use widgets::{MainBar, MainBarInit};

fn main() {
    let _ = relm4::RELM_THREADS.set(4);

    ShellApp::new("io.github.Locus.RsynapseShell")
        .with_scss(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/stylesheets/rsynapse-shell.scss"
        ))
        .watch_stylesheets(true)
        .on_startup(|_| {
            adw::init().expect("failed to initialize libadwaita");
            theme::prepare_theme();
        })
        .run_async::<MainBar>(MainBarInit {
            title: "Rsynapse Shell",
        });
}
