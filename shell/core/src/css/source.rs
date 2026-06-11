use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{StylesheetError, compiler::compile_scss};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StylesheetSource {
    Css(PathBuf),
    Scss(PathBuf),
}

impl StylesheetSource {
    pub fn css(path: impl Into<PathBuf>) -> Self {
        Self::Css(path.into())
    }

    pub fn scss(path: impl Into<PathBuf>) -> Self {
        Self::Scss(path.into())
    }

    pub fn load(&self) -> Result<String, StylesheetError> {
        match self {
            Self::Css(path) => fs::read_to_string(path).map_err(|source| StylesheetError::Read {
                path: path.clone(),
                source,
            }),
            Self::Scss(path) => compile_scss(path),
        }
    }

    pub fn watch_root(&self) -> PathBuf {
        match self {
            Self::Css(path) => path.clone(),
            Self::Scss(path) => path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(".")),
        }
    }
}
