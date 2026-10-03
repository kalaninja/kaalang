//! Serializes a call node.
use std::fmt::Write;

use super::{Node, TextAnchor, write_label};
use crate::layout::{CALL_BAR_INSET, NODE_LABEL_PADDING_X};

pub(super) fn name(label: &str) -> String {
    format!("Call: {label}")
}

pub(super) fn write(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    let bar = half_width - CALL_BAR_INSET;
    emit!(
        svg,
        "      <rect class=\"node-shape\" x=\"-{half_width}\" y=\"-{half_height}\" width=\"{}\" height=\"{}\"/>",
        node.width,
        node.height
    );
    // The bars inset from both sides are what separates a call from an action.
    emit!(
        svg,
        "      <path class=\"call-bars\" d=\"M -{bar} -{half_height} V {half_height} M {bar} -{half_height} V {half_height}\"/>"
    );
    write_label(svg, node, 0, -bar + NODE_LABEL_PADDING_X, TextAnchor::Start);
}
