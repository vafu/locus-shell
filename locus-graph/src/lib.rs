//! Typed Locus graph contracts and provider integration.
//!
//! The `binding`, `model`, and `paths` modules are generated from the Locus
//! schema. This crate owns the generated binding type and the D-Bus-backed
//! provider implementation for it.
//!
//! Verify the generated contracts with `sh scripts/locus-graph-schema check`
//! from the workspace root.

mod decode;
mod error;
mod generated;
mod watch;

#[cfg(test)]
mod test;

pub use decode::DecodeLocusValue;
pub(crate) use decode::decode_wire_field;
pub use error::{DecodeError, WatchError};
pub use generated::{binding, model, paths};
pub use watch::watch_field;

pub type FieldBinding<T> = binding::FieldBinding<T>;
