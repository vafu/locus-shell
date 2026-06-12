//! Generic Locus graph binding types and provider integration.
//!
//! This crate owns the reusable Locus-over-D-Bus provider implementation.
//! Schema-specific model markers, path constants, relation constants, and
//! convenience extension traits are generated in consuming crates.

mod binding;
mod collection;
mod decode;
mod error;
mod node;
mod watch;

#[cfg(test)]
mod test;

pub use binding::{FieldBinding, Path, Property};
pub use collection::{
    KindFilteredNodeListBinding, NodeId, NodeListBinding, NodeListDiffCommand, Relation,
    TargetBinding, watch_node_list, watch_target,
};
pub use decode::DecodeLocusValue;
pub(crate) use decode::decode_wire_field;
pub use error::{DecodeError, ListError, WatchError};
pub use node::{NodePropertyBinding, NodeRef, node};
pub use watch::watch_field;
