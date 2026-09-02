//! Serializes an action node.

use std::fmt::Write;

use super::{Node, write_label};

pub(super) fn name(node: &Node) -> String {
    format!("Action: {}", node.label)
}

pub(super) fn write(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    emit!(
        svg,
        "      <rect class=\"node-shape\" x=\"-{}\" y=\"-{}\" width=\"{}\" height=\"{}\"/>",
        half_width,
        half_height,
        node.width,
        node.height
    );
    write_label(svg, node, 0, -node.width / 2 + 16);
}
