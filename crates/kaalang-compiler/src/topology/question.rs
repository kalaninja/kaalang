//! Projects a question's node and one exit per branch output.

use crate::model::Block;

use super::{Exit, Node, NodeKind, block_node, branch_exits, drawn_branch_exit};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    debug_assert_eq!(block.question_branches.len(), block.outputs.len());
    nodes.push(block_node(index, NodeKind::Question));
    exits.extend(branch_exits(index, block, drawn_branch_exit));
}
