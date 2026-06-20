use std::{
    io::{self, ErrorKind},
    path::Path,
};

pub(super) fn is_missing(error: &io::Error) -> bool {
    matches!(error.kind(), ErrorKind::NotFound)
}

pub(super) fn watch_error(operation: &str, path: &Path, error: io::Error) -> String {
    format!("{operation} for {} failed: {error}", path.display())
}
