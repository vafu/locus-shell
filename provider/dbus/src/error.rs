use std::{error::Error as StdError, fmt, sync::Arc};

#[derive(Clone, Debug)]
pub enum WatchError {
    Fdo(Arc<zbus::fdo::Error>),
    Zbus(Arc<zbus::Error>),
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fdo(error) => write!(f, "{error}"),
            Self::Zbus(error) => write!(f, "{error}"),
        }
    }
}

impl StdError for WatchError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Fdo(error) => Some(error.as_ref()),
            Self::Zbus(error) => Some(error.as_ref()),
        }
    }
}

impl From<zbus::Error> for WatchError {
    fn from(error: zbus::Error) -> Self {
        Self::Zbus(Arc::new(error))
    }
}

impl From<zbus::fdo::Error> for WatchError {
    fn from(error: zbus::fdo::Error) -> Self {
        Self::Fdo(Arc::new(error))
    }
}
