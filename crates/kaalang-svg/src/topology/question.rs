//! Projects a question's node and one exit per branch output.

use kaalang_model::Block;

use super::{Exit, ExitId, Node, NodeId, NodeKind, block_node, provided};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    debug_assert_eq!(block.question_branches.len(), block.outputs.len());
    nodes.push(block_node(index, block, NodeKind::Question));
    exits.extend(
        block
            .outputs
            .iter()
            .zip(&block.question_branches)
            .enumerate()
            .map(|(branch, (_, answer))| Exit {
                id: exit(index, branch),
                handover: vec![provided(block.output_binding(branch))],
                branch_description: answer.description.clone(),
            }),
    );
}

/// One branch's exit. A question is drawn as a single node, so the branch is
/// named in the exit rather than in the node.
pub(crate) const fn exit(index: usize, branch: usize) -> ExitId {
    ExitId {
        node: NodeId::Block(index),
        branch: Some(branch),
    }
}
