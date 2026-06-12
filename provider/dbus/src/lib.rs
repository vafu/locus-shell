mod error;
mod property;
mod watch;

#[cfg(test)]
mod test;

pub use error::WatchError;
pub use property::{DbusBus, DbusPropertyKey, Object, PropertyBinding};
pub use property_provider::Property;
pub use watch::watch_property;
