use std::{error::Error as StdError, fmt, num};

#[derive(Debug)]
pub enum WatchError {
    Decode(DecodeError),
    List(ListError),
    Fdo(zbus::fdo::Error),
    Zbus(zbus::Error),
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "{error}"),
            Self::List(error) => write!(f, "{error}"),
            Self::Fdo(error) => write!(f, "{error}"),
            Self::Zbus(error) => write!(f, "{error}"),
        }
    }
}

impl StdError for WatchError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::List(error) => Some(error),
            Self::Fdo(error) => Some(error),
            Self::Zbus(error) => Some(error),
        }
    }
}

impl From<DecodeError> for WatchError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}

impl From<ListError> for WatchError {
    fn from(error: ListError) -> Self {
        Self::List(error)
    }
}

impl From<zbus::Error> for WatchError {
    fn from(error: zbus::Error) -> Self {
        Self::Zbus(error)
    }
}

impl From<zbus::fdo::Error> for WatchError {
    fn from(error: zbus::fdo::Error) -> Self {
        Self::Fdo(error)
    }
}

#[derive(Debug)]
pub enum DecodeError {
    MissingValue,
    Bool {
        value: String,
    },
    U32 {
        value: String,
        source: num::ParseIntError,
    },
    I32 {
        value: String,
        source: num::ParseIntError,
    },
    F64 {
        value: String,
        source: num::ParseFloatError,
    },
}

#[derive(Debug)]
pub enum ListError {
    UnknownCommand {
        command: String,
    },
    AddIndexOutOfBounds {
        node: String,
        index: usize,
        len: usize,
    },
    RemoveIndexOutOfBounds {
        node: String,
        index: usize,
        len: usize,
    },
    RemovedNodeMismatch {
        expected: String,
        actual: String,
        index: usize,
    },
}

impl fmt::Display for ListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand { command } => {
                write!(f, "unknown Locus node-list diff command: {command:?}")
            }
            Self::AddIndexOutOfBounds { node, index, len } => write!(
                f,
                "cannot add Locus node {node:?} at index {index}; list length is {len}"
            ),
            Self::RemoveIndexOutOfBounds { node, index, len } => write!(
                f,
                "cannot remove Locus node {node:?} at index {index}; list length is {len}"
            ),
            Self::RemovedNodeMismatch {
                expected,
                actual,
                index,
            } => write!(
                f,
                "cannot remove Locus node {expected:?} at index {index}; found {actual:?}"
            ),
        }
    }
}

impl StdError for ListError {}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue => write!(f, "missing Locus value for non-optional property"),
            Self::Bool { value } => write!(f, "invalid bool value from Locus: {value:?}"),
            Self::U32 { value, .. } => write!(f, "invalid u32 value from Locus: {value:?}"),
            Self::I32 { value, .. } => write!(f, "invalid i32 value from Locus: {value:?}"),
            Self::F64 { value, .. } => write!(f, "invalid f64 value from Locus: {value:?}"),
        }
    }
}

impl StdError for DecodeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::MissingValue => None,
            Self::Bool { .. } => None,
            Self::U32 { source, .. } => Some(source),
            Self::I32 { source, .. } => Some(source),
            Self::F64 { source, .. } => Some(source),
        }
    }
}
