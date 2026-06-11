use std::{path::Path, process::Command};

use super::StylesheetError;

pub(super) fn compile_scss(path: &Path) -> Result<String, StylesheetError> {
    let output = Command::new("sass")
        .arg("--no-source-map")
        .arg(path)
        .output()
        .map_err(|source| StylesheetError::SpawnSass { source })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(StylesheetError::CompileScss {
            path: path.to_path_buf(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}
