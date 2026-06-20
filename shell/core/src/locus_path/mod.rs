use std::path::{Path, PathBuf};

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
        Self { path: path.into() }
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
