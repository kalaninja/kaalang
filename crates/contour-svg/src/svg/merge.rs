//! Serializes and names a merge node.

use std::fmt::Write;

use super::{Node, NodeId, NodeKind, Scene};

pub(super) fn name(scene: &Scene, node: &Node) -> String {
    match node.id {
        NodeId::Block(index) => format!("Merge {}", ordinal(scene, index)),
        NodeId::Start | NodeId::Case { .. } | NodeId::Return => {
            unreachable!("merge nodes are authored blocks")
        }
    }
}

/// Numbers a merge by its position among the flow's merges. A block index would
/// name a count the reader cannot see: the node itself is drawn only as "M".
fn ordinal(scene: &Scene, index: usize) -> usize {
    scene
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Merge)
        .filter(|node| matches!(node.id, NodeId::Block(other) if other <= index))
        .count()
}

pub(super) fn write(svg: &mut String, node: &Node) {
    emit!(
        svg,
        "      <circle class=\"node-shape\" r=\"{}\"/>",
        node.width / 2
    );
    svg.push_str("      <text class=\"label\" y=\"5\">M</text>\n");
}
