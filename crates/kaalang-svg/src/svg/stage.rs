//! The transition node mirrors a stage's case-shaped entry vertically.

use std::fmt::Write;

use crate::layout::CASE_TIP_HEIGHT;
use kaalang_compiler::topology::NodeKind;

use super::{Node, TextAnchor, write_label};

pub(super) fn write_transition(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    let body_top = -half_height + CASE_TIP_HEIGHT;
    emit!(
        svg,
        "      <polygon class=\"node-shape\" points=\"0,-{half_height} {half_width},{body_top} {half_width},{half_height} -{half_width},{half_height} -{half_width},{body_top}\"/>"
    );
    write_label(svg, node, CASE_TIP_HEIGHT / 2, 0, TextAnchor::Middle);
}

pub(super) fn write_marker(svg: &mut String, node: &Node, kind: NodeKind) {
    let half_width = node.width / 4;
    let tip = node.height / 2;
    let base = tip - CASE_TIP_HEIGHT / 2;
    match kind {
        NodeKind::StageEntry => emit!(
            svg,
            "      <polygon class=\"stage-marker\" points=\"0,{tip} -{half_width},{base} {half_width},{base}\"/>"
        ),
        NodeKind::Transition => emit!(
            svg,
            "      <polygon class=\"stage-marker\" points=\"0,-{tip} -{half_width},-{base} {half_width},-{base}\"/>"
        ),
        _ => unreachable!("only stage entries and transitions carry markers"),
    }
}
