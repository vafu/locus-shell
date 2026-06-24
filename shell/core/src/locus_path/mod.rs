use std::path::{Component, Path, PathBuf};

/// Convenience builder for paths inside a locusfs mount.
///
/// This type is intentionally independent from `source`: source functions
/// accept `impl Into<PathBuf>`, and `LocusPath` is just an ergonomic way to
/// produce those paths.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LocusPath {
    path: PathBuf,
}

impl LocusPath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: normalize_path(path.into()),
        }
    }

    pub fn from_env_or(env: &str, default: impl Into<PathBuf>) -> Self {
        std::env::var_os(env)
            .map(PathBuf::from)
            .map(Self::new)
            .unwrap_or_else(|| Self::new(default))
    }

    pub fn node(&self, node: &str) -> Self {
        let (kind, local) = node.split_once(':').unwrap_or(("node", node));
        self.child(kind).child(local)
    }

    pub fn child(&self, child: impl AsRef<Path>) -> Self {
        Self::new(self.path.join(child))
    }

    /// Appends one locusfs filesystem segment, percent-encoding bytes that are
    /// not valid as plain path segment bytes in the FUSE layout.
    pub fn encoded_child(&self, child: impl AsRef<str>) -> Self {
        self.child(encode_segment(child.as_ref()))
    }

    pub fn prop(&self, property: impl AsRef<Path>) -> Self {
        self.child(property)
    }

    pub fn rel(&self, relation: impl AsRef<Path>) -> Self {
        self.child(relation)
    }

    pub fn relation(&self, relation: impl AsRef<Path>) -> Self {
        self.rel(relation)
    }

    pub fn node_id(&self) -> Result<String, String> {
        node_id_from_path(&self.path)
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    normalized
}

impl AsRef<Path> for LocusPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<LocusPath> for PathBuf {
    fn from(path: LocusPath) -> Self {
        path.into_path_buf()
    }
}

impl From<PathBuf> for LocusPath {
    fn from(path: PathBuf) -> Self {
        Self::new(path)
    }
}

impl From<&Path> for LocusPath {
    fn from(path: &Path) -> Self {
        Self::new(path)
    }
}

impl From<&LocusPath> for PathBuf {
    fn from(path: &LocusPath) -> Self {
        path.path.clone()
    }
}

pub fn node_id_from_path(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    let local = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid node path: {}", path.display()))?;
    let kind = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid node path: {}", path.display()))?;

    Ok(format!("{kind}:{local}"))
}

/// Percent-encodes one raw locusfs path segment using the FUSE layout rules.
pub fn encode_segment(raw: &str) -> String {
    let mut encoded = String::new();
    for byte in raw.bytes() {
        if is_plain_segment_byte(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn is_plain_segment_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!("nibble is always <= 15"),
    }
}

#[cfg(test)]
mod tests {
    use super::{LocusPath, encode_segment};

    #[test]
    fn encodes_shell_hostile_segments() {
        assert_eq!(
            encode_segment("1_9976:MenuBar:0000:3"),
            "1_9976%3AMenuBar%3A0000%3A3"
        );
    }

    #[test]
    fn appends_encoded_child_segment() {
        assert_eq!(
            LocusPath::new("/tmp/locusfs/dbusmenu/item")
                .encoded_child("1_9976:MenuBar")
                .as_path(),
            std::path::Path::new("/tmp/locusfs/dbusmenu/item/1_9976%3AMenuBar")
        );
    }

    #[test]
    fn normalizes_relative_parent_segments_without_touching_filesystem() {
        assert_eq!(
            LocusPath::new("/run/user/1000/locusfs/window/5/../../app-instance/session").as_path(),
            std::path::Path::new("/run/user/1000/locusfs/app-instance/session")
        );
    }
}
