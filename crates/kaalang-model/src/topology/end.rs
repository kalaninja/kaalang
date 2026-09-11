//! Projects the implicit end node.

use super::{Node, NodeKind, block_node};

/// End has no exits and no description of its own: its caption is the flow's
/// return type, which a presentation derives from the authored source
/// (RFC 0002 §4.6).
pub(super) fn project(index: usize, nodes: &mut Vec<Node>) {
    nodes.push(block_node(index, NodeKind::End));
}
