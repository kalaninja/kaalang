//! Projects the implicit end node.

use kaalang_model::Block;

use super::{Node, NodeKind, block_node};

/// End has no exits and no description of its own: its caption is the flow's
/// return type, as RFC 0002 §4.6 requires.
pub(super) fn project(index: usize, block: &Block, return_type: &str, nodes: &mut Vec<Node>) {
    nodes.push(Node {
        label: return_type.to_owned(),
        ..block_node(index, block, NodeKind::End)
    });
}
