//! Projects a call's node and its single exit.

use crate::model::Block;

use super::{Exit, Node, NodeKind, block_node, sequential_exit};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    nodes.push(block_node(index, NodeKind::Call));
    exits.push(sequential_exit(index, block));
}
