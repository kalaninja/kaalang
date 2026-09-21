//! Serializes a question node.
use std::fmt::Write;

use crate::layout::QUESTION_POINT;

use super::{Node, TextAnchor, write_label};

pub(super) fn name(label: &str) -> String {
    format!("Question: {label}")
}

pub(super) fn write(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    let inner = half_width - QUESTION_POINT;
    emit!(
        svg,
        "      <polygon class=\"node-shape\" points=\"-{inner},-{half_height} {inner},-{half_height} {half_width},0 {inner},{half_height} -{inner},{half_height} -{half_width},0\"/>"
    );
    write_label(svg, node, 0, 0, TextAnchor::Middle);
}
