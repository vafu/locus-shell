use std::{error::Error as StdError, fmt, num};

#[derive(Debug)]
pub enum WatchError {
    Decode(DecodeError),
    Fdo(zbus::fdo::Error),
    Zbus(zbus::Error),
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "{error}"),
            Self::Fdo(error) => write!(f, "{error}"),
            Self::Zbus(error) => write!(f, "{error}"),
        }
    }
}

impl StdError for WatchError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
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

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::Bool { .. } => None,
            Self::U32 { source, .. } => Some(source),
            Self::I32 { source, .. } => Some(source),
            Self::F64 { source, .. } => Some(source),
        }
    }
}
