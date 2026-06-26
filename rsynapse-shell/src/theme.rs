use std::{
    cell::RefCell,
    fs, io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
};

use shell_core::gtk::{self, gio, prelude::*};

const MATERIAL_ICON_THEME: &str = "Material";
const INTERFACE_SCHEMA: &str = "org.gnome.desktop.interface";
const LIGHT_SCHEME: &str = "prefer-light";
const DARK_SCHEME: &str = "prefer-dark";

pub(crate) fn prepare_theme() {
    prepare_desktop_theme();
    prepare_accent_sync();
    sync_color_scheme();
    sync_accent_color();
    prepare_icons();
}

pub(crate) fn toggle_color_scheme() -> Result<(), String> {
    let settings = interface_settings();
    let next = toggled_color_scheme(settings.string("color-scheme").as_str());
    settings
        .set_string("color-scheme", next)
        .map_err(|error| format!("set color-scheme to {next}: {error}"))?;
    update_gtk_theme(&settings, next)
}

fn prepare_icons() {
    if let Some(display) = gtk::gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);
        icon_theme.add_search_path(icon_search_path());
        icon_theme.set_theme_name(Some(MATERIAL_ICON_THEME));
    }

    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_icon_theme_name(Some(MATERIAL_ICON_THEME));
    }
}

fn prepare_desktop_theme() {
    let settings = interface_settings();
    settings.connect_changed(Some("color-scheme"), |settings, _| {
        if let Err(error) = update_gtk_theme(settings, settings.string("color-scheme").as_str()) {
            eprintln!("[theme] failed to update GTK theme: {error}");
        }
    });
    SETTINGS.with(|cell| {
        *cell.borrow_mut() = Some(settings);
    });
}

fn prepare_accent_sync() {
    let settings = interface_settings();
    settings.connect_changed(Some("accent-color"), |settings, _| {
        if let Err(error) = sync_accent(settings.string("accent-color").as_str()) {
            eprintln!("[theme] failed to sync accent color: {error}");
        }
    });
    ACCENT_SETTINGS.with(|cell| {
        *cell.borrow_mut() = Some(settings);
    });
}

fn sync_color_scheme() {
    let settings = interface_settings();
    if let Err(error) = update_gtk_theme(&settings, settings.string("color-scheme").as_str()) {
        eprintln!("[theme] failed to sync GTK theme: {error}");
    }
}

fn sync_accent_color() {
    let settings = interface_settings();
    if let Err(error) = sync_accent(settings.string("accent-color").as_str()) {
        eprintln!("[theme] failed to sync accent color: {error}");
    }
}

fn update_gtk_theme(settings: &gio::Settings, color_scheme: &str) -> Result<(), String> {
    let current_theme = settings.string("gtk-theme");
    let (theme, style) = theme_for_scheme(current_theme.as_str(), color_scheme);
    let (source, target) = niri_theme_paths(&config_home(), style);
    replace_symlink(&source, &target).map_err(|error| {
        format!(
            "link niri theme {} -> {}: {error}",
            target.display(),
            source.display()
        )
    })?;
    settings
        .set_string("gtk-theme", &theme)
        .map_err(|error| format!("set gtk-theme to {theme}: {error}"))
}

fn sync_accent(color: &str) -> Result<(), String> {
    let script = config_home().join("ags/scripts/sync_accent.sh");
    if !script.exists() {
        return Ok(());
    }
    let status = Command::new("bash")
        .arg(&script)
        .arg(color)
        .status()
        .map_err(|error| format!("run {}: {error}", script.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with {status}", script.display()))
    }
}

fn replace_symlink(source: &Path, target: &Path) -> io::Result<()> {
    match fs::remove_file(target) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    symlink(source, target)
}

fn theme_for_scheme(current_theme: &str, color_scheme: &str) -> (String, ThemeStyle) {
    let base = current_theme.replace("-dark", "");
    if color_scheme == DARK_SCHEME {
        (format!("{base}-dark"), ThemeStyle::Dark)
    } else {
        (base, ThemeStyle::Light)
    }
}

fn toggled_color_scheme(current: &str) -> &'static str {
    if current == LIGHT_SCHEME {
        DARK_SCHEME
    } else {
        LIGHT_SCHEME
    }
}

fn niri_theme_paths(config_home: &Path, style: ThemeStyle) -> (PathBuf, PathBuf) {
    let style = match style {
        ThemeStyle::Light => "light",
        ThemeStyle::Dark => "dark",
    };
    let niri = config_home.join("niri");
    (
        niri.join(format!("theme_{style}.kdl")),
        niri.join("theme.kdl"),
    )
}

fn interface_settings() -> gio::Settings {
    gio::Settings::new(INTERFACE_SCHEMA)
}

fn icon_search_path() -> PathBuf {
    data_home().join("icons")
}

fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThemeStyle {
    Light,
    Dark,
}

thread_local! {
    static SETTINGS: RefCell<Option<gio::Settings>> = const { RefCell::new(None) };
    static ACCENT_SETTINGS: RefCell<Option<gio::Settings>> = const { RefCell::new(None) };
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ThemeStyle, niri_theme_paths, theme_for_scheme, toggled_color_scheme};

    #[test]
    fn toggles_color_scheme_like_ags() {
        assert_eq!(toggled_color_scheme("prefer-light"), "prefer-dark");
        assert_eq!(toggled_color_scheme("prefer-dark"), "prefer-light");
        assert_eq!(toggled_color_scheme("default"), "prefer-light");
    }

    #[test]
    fn derives_gtk_theme_name_from_scheme() {
        assert_eq!(
            theme_for_scheme("Adwaita", "prefer-dark"),
            ("Adwaita-dark".to_owned(), ThemeStyle::Dark)
        );
        assert_eq!(
            theme_for_scheme("Adwaita-dark", "prefer-light"),
            ("Adwaita".to_owned(), ThemeStyle::Light)
        );
    }

    #[test]
    fn derives_niri_theme_symlink_paths() {
        let (source, target) = niri_theme_paths(Path::new("/tmp/config"), ThemeStyle::Dark);

        assert_eq!(source, Path::new("/tmp/config/niri/theme_dark.kdl"));
        assert_eq!(target, Path::new("/tmp/config/niri/theme.kdl"));
    }
}
