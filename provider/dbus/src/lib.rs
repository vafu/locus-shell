mod error;
mod property;
mod watch;

#[cfg(test)]
mod test;

pub use error::WatchError;
pub use property::{DbusBus, Object, Property, PropertyBinding};
pub use watch::watch_property;
