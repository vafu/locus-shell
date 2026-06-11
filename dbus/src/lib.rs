mod decode;
mod error;
mod property;
mod watch;

pub mod schema;

#[cfg(test)]
mod test;

pub use decode::DecodeLocusValue;
pub(crate) use decode::decode_wire_field;
pub use error::{DecodeError, WatchError};
pub use property::{DbusBus, Object, Property, PropertyBinding};
pub use watch::{watch_field, watch_property};

pub type FieldBinding<T> = schema::binding::FieldBinding<T>;
