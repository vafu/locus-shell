//! Typed Locus graph contracts and provider integration.
//!
//! The `binding`, `model`, and `paths` modules are generated from the Locus
//! schema. This crate owns the generated binding type and the D-Bus-backed
//! provider implementation for it.
//!
//! Verify the generated contracts with `sh scripts/locus-provider-schema check`
//! from the workspace root.

mod collection;
mod decode;
mod error;
mod generated;
mod node;
mod watch;

#[cfg(test)]
mod test;

pub use collection::{
    NodeId, NodeListBinding, NodeListDiffCommand, Relation, TargetBinding,
    WorkspaceWindowsProvider, relations, watch_node_list, watch_target,
};
pub use decode::DecodeLocusValue;
pub(crate) use decode::decode_wire_field;
pub use error::{DecodeError, ListError, WatchError};
pub use generated::{binding, model, paths};
pub use node::{NodePropertyBinding, NodeRef, node};
pub use watch::watch_field;

pub type FieldBinding<T> = binding::FieldBinding<T>;
