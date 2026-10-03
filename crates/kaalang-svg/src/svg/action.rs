//! Serializes an action node.
use std::fmt::Write;

use super::{Node, TextAnchor, write_label};
use crate::layout::NODE_LABEL_PADDING_X;

pub(super) fn name(label: &str) -> String {
    format!("Action: {label}")
}

pub(super) fn write(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    emit!(
        svg,
        "      <rect class=\"node-shape\" x=\"-{half_width}\" y=\"-{half_height}\" width=\"{}\" height=\"{}\"/>",
        node.width,
        node.height
    );
    write_label(
        svg,
        node,
        0,
        -half_width + NODE_LABEL_PADDING_X,
        TextAnchor::Start,
    );
}
