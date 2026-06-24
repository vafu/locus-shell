use std::{fs, thread};

use shell_core::source::{self, Observable, rx::Observable as _};

const POWER_PROFILE_PROPERTIES_PATH: &str = "dbus/powerprofiles/object/@/@properties";
const POWER_PROFILE_ORDER: &[&str] = &["power-saver", "balanced", "performance"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PowerProfileView {
    pub(super) visible: bool,
    pub(super) profile: String,
    pub(super) icon: &'static str,
    pub(super) tooltip: String,
}

impl Default for PowerProfileView {
    fn default() -> Self {
        Self {
            visible: false,
            profile: String::new(),
            icon: "speed",
            tooltip: String::new(),
        }
    }
}

pub(super) fn power_profile_status() -> Observable<PowerProfileView> {
    source::root()
        .child(POWER_PROFILE_PROPERTIES_PATH)
        .observe_prop_or::<String>("ActiveProfile", String::new())
        .map(power_profile_view)
        .distinct_until_changed()
        .box_it()
}

pub(super) fn cycle_power_profile(profile: &str) {
    let next = next_profile(profile).to_owned();
    let path = source::root()
        .child(POWER_PROFILE_PROPERTIES_PATH)
        .prop("ActiveProfile")
        .into_path_buf();

    thread::spawn(move || {
        if let Err(error) = fs::write(&path, next) {
            eprintln!(
                "[power-profile] failed to write {}: {error}",
                path.display()
            );
        }
    });
}

fn power_profile_view(profile: String) -> PowerProfileView {
    let profile = profile.trim().to_owned();
    if profile.is_empty() {
        return PowerProfileView::default();
    }

    PowerProfileView {
        visible: true,
        tooltip: tooltip(&profile),
        icon: icon_name(&profile),
        profile,
    }
}

fn icon_name(profile: &str) -> &'static str {
    match profile {
        "performance" => "bolt",
        "power-saver" => "eco",
        _ => "speed",
    }
}

fn tooltip(profile: &str) -> String {
    match profile {
        "performance" => "Performance".to_owned(),
        "power-saver" => "Power Saver".to_owned(),
        "balanced" => "Balanced".to_owned(),
        _ => profile.to_owned(),
    }
}

fn next_profile(profile: &str) -> &'static str {
    let current = POWER_PROFILE_ORDER
        .iter()
        .position(|candidate| *candidate == profile)
        .unwrap_or(1);
    POWER_PROFILE_ORDER[(current + 1) % POWER_PROFILE_ORDER.len()]
}
