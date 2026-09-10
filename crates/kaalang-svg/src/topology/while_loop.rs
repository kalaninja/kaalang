//! Projects while conditions as question nodes.

use kaalang_model::Block;

use super::{Exit, Node, NodeKind, block_node, question};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    nodes.push(block_node(index, block, NodeKind::Question));
    exits.extend(
        block
            .question_branches
            .iter()
            .enumerate()
            .map(|(branch, answer)| Exit {
                id: question::exit(index, branch),
                handover: Vec::new(),
                branch_description: Some(
                    answer
                        .description
                        .clone()
                        .unwrap_or_else(|| if answer.is_yes { "YES" } else { "NO" }.to_owned()),
                ),
            }),
    );
}
