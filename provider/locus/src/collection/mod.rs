mod binding;
mod diff;
mod watch;

#[cfg(test)]
mod test;

pub use binding::{NodeId, NodeListBinding, Relation, TargetBinding, relations};
pub use diff::NodeListDiffCommand;
pub use watch::{watch_node_list, watch_target};
