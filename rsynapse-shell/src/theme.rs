use std::path::PathBuf;

use shell_core::gtk;

const MATERIAL_ICON_THEME: &str = "Material";

pub(crate) fn prepare_theme() {
    if let Some(display) = gtk::gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);
        icon_theme.add_search_path(icon_search_path());
        icon_theme.set_theme_name(Some(MATERIAL_ICON_THEME));
    }

    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_icon_theme_name(Some(MATERIAL_ICON_THEME));
    }
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
