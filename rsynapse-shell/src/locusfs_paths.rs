use std::path::{Path, PathBuf};

use shell_core::{locus_path::LocusPath, source};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DbusServicePath {
    local_id: &'static str,
    object_manager_path: &'static str,
}

pub(crate) const AGENTDBUS: DbusServicePath =
    DbusServicePath::new("agentdbus", "/io/github/AgentDBus");
pub(crate) const BLUEZ: DbusServicePath = DbusServicePath::new("bluez", "/");
pub(crate) const NETWORK_MANAGER: DbusServicePath =
    DbusServicePath::new("networkmanager", "/org/freedesktop/NetworkManager");
pub(crate) const POWER_PROFILES: DbusServicePath =
    DbusServicePath::new("powerprofiles", "/net/hadess/PowerProfiles");
pub(crate) const UPOWER: DbusServicePath =
    DbusServicePath::new("upower", "/org/freedesktop/UPower");

impl DbusServicePath {
    pub(crate) const fn new(local_id: &'static str, object_manager_path: &'static str) -> Self {
        Self {
            local_id,
            object_manager_path,
        }
    }

    pub(crate) fn objects(self) -> LocusPath {
        self.service_root().child("objects")
    }

    pub(crate) fn methods(self) -> LocusPath {
        self.service_root().child("methods")
    }

    pub(crate) fn object(self, relative: impl AsRef<Path>) -> LocusPath {
        append_relative(self.objects(), relative.as_ref())
    }

    pub(crate) fn object_from_dbus_path(self, path: &str) -> Option<LocusPath> {
        let relative = relative_object_path(self.object_manager_path, path)?;
        Some(append_relative(self.objects(), &relative))
    }

    pub(crate) fn method_for_object(self, object: &LocusPath, method: &str) -> Option<LocusPath> {
        let objects = self.objects();
        let relative = object.as_path().strip_prefix(objects.as_path()).ok()?;
        Some(append_relative(self.methods(), relative).child(method))
    }

    fn service_root(self) -> LocusPath {
        source::root().child("dbus").child(self.local_id)
    }
}

fn append_relative(base: LocusPath, relative: &Path) -> LocusPath {
    if relative.as_os_str().is_empty() {
        base
    } else {
        base.child(relative)
    }
}

fn relative_object_path(object_manager_path: &str, path: &str) -> Option<PathBuf> {
    if !path.starts_with('/') {
        return None;
    }

    if path == "/" {
        return (object_manager_path == "/").then(PathBuf::new);
    }

    if path == object_manager_path {
        return Some(PathBuf::new());
    }

    if object_manager_path == "/" {
        return Some(dbus_path_segments(path.trim_start_matches('/')));
    }

    let manager_prefix = format!("{}/", object_manager_path.trim_end_matches('/'));
    if let Some(relative) = path.strip_prefix(manager_prefix.as_str()) {
        return Some(dbus_path_segments(relative));
    }

    let mut absolute = PathBuf::from("_absolute");
    absolute.push(dbus_path_segments(path.trim_start_matches('/')));
    Some(absolute)
}

fn dbus_path_segments(path: &str) -> PathBuf {
    let mut output = PathBuf::new();
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        output.push(segment);
    }
    output
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::relative_object_path;

    #[test]
    fn maps_object_manager_root_to_object_tree_root() {
        assert_eq!(
            relative_object_path("/net/hadess/PowerProfiles", "/net/hadess/PowerProfiles").unwrap(),
            Path::new("")
        );
    }

    #[test]
    fn strips_object_manager_prefix() {
        assert_eq!(
            relative_object_path(
                "/org/freedesktop/UPower",
                "/org/freedesktop/UPower/devices/battery_BAT1"
            )
            .unwrap(),
            Path::new("devices/battery_BAT1")
        );
    }

    #[test]
    fn root_object_manager_uses_path_relative_to_objects_root() {
        assert_eq!(
            relative_object_path("/", "/org/bluez/hci0/dev_00_11").unwrap(),
            Path::new("org/bluez/hci0/dev_00_11")
        );
    }

    #[test]
    fn outside_manager_paths_use_absolute_namespace() {
        assert_eq!(
            relative_object_path("/org/example/Manager", "/outside/object").unwrap(),
            Path::new("_absolute/outside/object")
        );
    }

    #[test]
    fn slash_is_absent_for_non_root_object_manager() {
        assert!(relative_object_path("/org/example/Manager", "/").is_none());
    }
}
