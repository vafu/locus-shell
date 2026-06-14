use shell_core::gtk::{self, prelude::*};

const DEFAULT_SIZE: u16 = 24;
const DEFAULT_STYLE: &str = "outlined";

pub(super) fn image(icon: &str) -> gtk::Image {
    let image = gtk::Image::new();
    image.add_css_class("materialicon");
    set_icon(&image, icon);
    image
}

pub(super) fn set_icon(image: &gtk::Image, icon: &str) {
    image.set_icon_name(Some(icon_name(icon).as_str()));
}

pub(super) fn icon_name(icon: &str) -> String {
    if icon.is_empty() || icon.ends_with("-symbolic") {
        icon.to_owned()
    } else {
        let material_name = material_icon_name(icon);
        if material_icon_exists(&material_name) {
            material_name
        } else {
            format!("{icon}-{DEFAULT_STYLE}-symbolic")
        }
    }
}

fn material_icon_name(icon: &str) -> String {
    format!("{icon}_fill1_{DEFAULT_SIZE}px-{DEFAULT_STYLE}-symbolic")
}

fn material_icon_exists(icon_name: &str) -> bool {
    data_home()
        .join("icons/Material/symbolic")
        .join(format!("{icon_name}.svg"))
        .exists()
}

fn data_home() -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".local/share"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".local/share"))
}
