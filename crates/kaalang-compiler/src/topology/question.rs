//! Projects a question's node and one exit per branch output.

use crate::model::Block;

use super::{Exit, ExitId, Node, NodeId, NodeKind, block_node, branch_exits};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    debug_assert_eq!(block.question_branches.len(), block.outputs.len());
    nodes.push(block_node(index, NodeKind::Question));
    exits.extend(branch_exits(index, block, exit));
}

/// One branch's exit. A question is drawn as a single node, so the branch is
/// named in the exit rather than in the node.
pub(super) const fn exit(index: usize, branch: usize) -> ExitId {
    ExitId {
        node: NodeId::Block(index),
        branch: Some(branch),
    }
}
