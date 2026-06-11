use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{StylesheetSource, fingerprint::stylesheet_fingerprint};

#[test]
fn scss_source_watches_parent_directory() {
    let source = StylesheetSource::scss("styles/main.scss");

    assert_eq!(source.watch_root(), PathBuf::from("styles"));
}

#[test]
fn stylesheet_fingerprint_tracks_stylesheet_path_changes() {
    let root = TempDir::new("stylesheet-path-changes");
    root.write("main.scss", "$color: red;");
    root.write("unused.scss", "$unused: blue;");

    let initial = stylesheet_fingerprint(root.path()).unwrap();

    root.remove("unused.scss");

    let updated = stylesheet_fingerprint(root.path()).unwrap();

    assert_ne!(initial, updated);
}

#[test]
fn stylesheet_fingerprint_ignores_unrelated_files() {
    let root = TempDir::new("stylesheet-unrelated-files");
    root.write("main.scss", "$color: red;");

    let initial = stylesheet_fingerprint(root.path()).unwrap();

    root.write("notes.txt", "not css");

    let updated = stylesheet_fingerprint(root.path()).unwrap();

    assert_eq!(initial, updated);
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "locus-shell-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn write(&self, name: &str, contents: &str) {
        fs::write(self.path.join(name), contents).unwrap();
    }

    fn remove(&self, name: &str) {
        fs::remove_file(self.path.join(name)).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
