//! Serializes a choice's Select node and its derived Case nodes.
use std::fmt::Write;

use crate::layout::{CASE_TIP_HEIGHT, SELECT_SKEW};

use super::{Node, TextAnchor, write_label};

pub(super) fn select_name(label: &str) -> String {
    format!("Select: {label}")
}

pub(super) fn case_name(label: &str) -> String {
    format!("Case: {label}")
}

pub(super) fn write_select(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    let inner = half_width - SELECT_SKEW;
    emit!(
        svg,
        "      <polygon class=\"node-shape\" points=\"-{inner},-{half_height} {half_width},-{half_height} {inner},{half_height} -{half_width},{half_height}\"/>"
    );
    write_label(svg, node, 0, 0, TextAnchor::Middle);
}

pub(super) fn write_case(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    let body_bottom = half_height - CASE_TIP_HEIGHT;
    emit!(
        svg,
        "      <polygon class=\"node-shape\" points=\"-{half_width},-{half_height} {half_width},-{half_height} {half_width},{body_bottom} 0,{half_height} -{half_width},{body_bottom}\"/>"
    );
    write_label(svg, node, -CASE_TIP_HEIGHT / 2, 0, TextAnchor::Middle);
}
