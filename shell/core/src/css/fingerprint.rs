use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use super::StylesheetError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) struct StylesheetFingerprint {
    newest_mtime: SystemTime,
    paths: Vec<PathBuf>,
}

pub(super) fn stylesheet_fingerprint(
    path: &Path,
) -> Result<StylesheetFingerprint, StylesheetError> {
    let metadata = fs::metadata(path).map_err(|source| StylesheetError::Watch {
        path: path.to_path_buf(),
        source,
    })?;

    if metadata.is_file() {
        let newest_mtime = metadata
            .modified()
            .map_err(|source| StylesheetError::Watch {
                path: path.to_path_buf(),
                source,
            })?;

        return Ok(StylesheetFingerprint {
            newest_mtime,
            paths: vec![path.to_path_buf()],
        });
    }

    let mut fingerprint = StylesheetFingerprint {
        newest_mtime: SystemTime::UNIX_EPOCH,
        paths: Vec::new(),
    };
    collect_stylesheet_fingerprint(path, &mut fingerprint)?;
    fingerprint.paths.sort();
    Ok(fingerprint)
}

fn collect_stylesheet_fingerprint(
    path: &Path,
    fingerprint: &mut StylesheetFingerprint,
) -> Result<(), StylesheetError> {
    for entry in fs::read_dir(path).map_err(|source| StylesheetError::Watch {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| StylesheetError::Watch {
            path: path.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|source| StylesheetError::Watch {
            path: path.clone(),
            source,
        })?;

        if metadata.is_dir() {
            collect_stylesheet_fingerprint(&path, fingerprint)?;
        } else if is_stylesheet_path(&path) {
            fingerprint.paths.push(path.clone());
            let modified = metadata
                .modified()
                .map_err(|source| StylesheetError::Watch {
                    path: path.to_path_buf(),
                    source,
                })?;
            if modified > fingerprint.newest_mtime {
                fingerprint.newest_mtime = modified;
            }
        }
    }

    Ok(())
}

fn is_stylesheet_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("css" | "scss" | "sass")
    )
}
