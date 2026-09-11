//! Projects an action's node and its single exit.

use crate::model::{Block, ProducerId};

use super::{Exit, ExitId, Node, NodeId, NodeKind, block_node};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    nodes.push(block_node(index, NodeKind::Action));
    exits.push(Exit {
        id: exit(index),
        provides: (0..block.outputs.len())
            .map(|output| ProducerId::BlockOutput {
                block: index,
                output,
            })
            .collect(),
    });
}

/// An action's only exit, which hands over every output it provides. It takes no
/// branch: an action scopes nothing per branch, so no output selects among exits
/// here.
pub(super) const fn exit(index: usize) -> ExitId {
    ExitId::of(NodeId::Block(index))
}
