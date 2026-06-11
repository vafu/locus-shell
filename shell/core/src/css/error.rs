use std::{fmt, io, path::PathBuf};

#[derive(Debug)]
pub enum StylesheetError {
    Read { path: PathBuf, source: io::Error },
    Watch { path: PathBuf, source: io::Error },
    SpawnSass { source: io::Error },
    CompileScss { path: PathBuf, message: String },
}

impl fmt::Display for StylesheetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::Watch { path, source } => {
                write!(formatter, "failed to inspect {}: {source}", path.display())
            }
            Self::SpawnSass { source } => {
                write!(formatter, "failed to run sass compiler: {source}")
            }
            Self::CompileScss { path, message } => {
                write!(formatter, "failed to compile {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for StylesheetError {}
