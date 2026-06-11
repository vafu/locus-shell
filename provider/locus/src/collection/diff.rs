use locus_dbus::NodeListDiffCommandTuple;

use crate::{ListError, collection::NodeId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeListDiffCommand {
    Reset { nodes: Vec<NodeId> },
    NodeAdded { node: NodeId, index: usize },
    NodeRemoved { node: NodeId, index: usize },
}

impl NodeListDiffCommand {
    pub(crate) fn from_tuple(
        (command, node, index, nodes): NodeListDiffCommandTuple,
    ) -> Result<Self, ListError> {
        match command.as_str() {
            "reset" => Ok(Self::Reset { nodes }),
            "node-added" => Ok(Self::NodeAdded {
                node,
                index: index as usize,
            }),
            "node-removed" => Ok(Self::NodeRemoved {
                node,
                index: index as usize,
            }),
            _ => Err(ListError::UnknownCommand { command }),
        }
    }
}

pub(crate) fn commands_from_tuples(
    commands: Vec<NodeListDiffCommandTuple>,
) -> Result<Vec<NodeListDiffCommand>, ListError> {
    commands
        .into_iter()
        .map(NodeListDiffCommand::from_tuple)
        .collect()
}

pub(crate) fn apply_commands(
    nodes: &mut Vec<NodeId>,
    commands: Vec<NodeListDiffCommand>,
) -> Result<(), ListError> {
    for command in commands {
        match command {
            NodeListDiffCommand::Reset { nodes: next } => {
                *nodes = next;
            }
            NodeListDiffCommand::NodeAdded { node, index } => {
                if index > nodes.len() {
                    return Err(ListError::AddIndexOutOfBounds {
                        node,
                        index,
                        len: nodes.len(),
                    });
                }
                nodes.insert(index, node);
            }
            NodeListDiffCommand::NodeRemoved { node, index } => {
                if index >= nodes.len() {
                    return Err(ListError::RemoveIndexOutOfBounds {
                        node,
                        index,
                        len: nodes.len(),
                    });
                }
                if nodes[index] != node {
                    return Err(ListError::RemovedNodeMismatch {
                        expected: node,
                        actual: nodes[index].clone(),
                        index,
                    });
                }
                nodes.remove(index);
            }
        }
    }

    Ok(())
}
