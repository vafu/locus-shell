use std::path::{Path, PathBuf};

use shell_core::{locus_path::LocusPath, source};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DbusBusPath {
    bus: &'static str,
}

pub(crate) const DBUS_SYSTEM: DbusBusPath = DbusBusPath::new("system");
pub(crate) const DBUS_SESSION: DbusBusPath = DbusBusPath::new("session");

impl DbusBusPath {
    pub(crate) const fn new(bus: &'static str) -> Self {
        Self { bus }
    }

    pub(crate) fn root(self) -> LocusPath {
        source::root().child("dbus").child(self.bus)
    }

    pub(crate) fn object(self, path: impl AsRef<str>) -> LocusPath {
        append_relative(self.root(), &dbus_path_segments(path.as_ref()))
    }

    pub(crate) fn object_from_dbus_path(self, path: &str) -> Option<LocusPath> {
        dbus_object_path(path).map(|path| append_relative(self.root(), &path))
    }

    pub(crate) fn method_for_object(self, object: &LocusPath, method: &str) -> Option<LocusPath> {
        object.as_path().strip_prefix(self.root().as_path()).ok()?;
        Some(object.child(method_call_name(method)))
    }
}

fn append_relative(base: LocusPath, relative: &Path) -> LocusPath {
    if relative.as_os_str().is_empty() {
        base
    } else {
        base.child(relative)
    }
}

fn dbus_object_path(path: &str) -> Option<PathBuf> {
    path.starts_with('/').then(|| dbus_path_segments(path))
}

fn dbus_path_segments(path: &str) -> PathBuf {
    let mut output = PathBuf::new();
    for segment in path
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
    {
        output.push(segment);
    }
    output
}

fn method_call_name(method: &str) -> String {
    format!("{method}.call")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use shell_core::{locus_path::LocusPath, source};

    use super::{DBUS_SESSION, DBUS_SYSTEM, dbus_object_path};

    #[test]
    fn maps_full_system_object_path_under_system_bus_root() {
        assert_eq!(
            DBUS_SYSTEM
                .object("/org/freedesktop/UPower/devices/DisplayDevice")
                .as_path(),
            source::root()
                .child("dbus/system/org/freedesktop/UPower/devices/DisplayDevice")
                .as_path()
        );
    }

    #[test]
    fn maps_full_session_object_path_under_session_bus_root() {
        assert_eq!(
            DBUS_SESSION
                .object("/io/github/AgentDBus/sessions/codex")
                .as_path(),
            source::root()
                .child("dbus/session/io/github/AgentDBus/sessions/codex")
                .as_path()
        );
    }

    #[test]
    fn dbus_root_object_maps_to_bus_root() {
        assert_eq!(dbus_object_path("/").unwrap(), Path::new(""));
    }

    #[test]
    fn rejects_non_absolute_dbus_object_paths() {
        assert!(dbus_object_path("org/freedesktop/UPower").is_none());
    }

    #[test]
    fn method_paths_append_call_suffix_to_object_directory() {
        let object = DBUS_SYSTEM.object("/org/bluez/hci0/dev_00_11");

        assert_eq!(
            DBUS_SYSTEM.method_for_object(&object, "Connect").unwrap(),
            object.child("Connect.call")
        );
    }

    #[test]
    fn method_paths_must_stay_under_matching_bus_root() {
        let object = LocusPath::new("/tmp/rsynapse/dbus/session/org/bluez/hci0/dev_00_11");

        assert!(DBUS_SYSTEM.method_for_object(&object, "Connect").is_none());
    }
}
