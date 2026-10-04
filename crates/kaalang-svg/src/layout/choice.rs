//! Sizes case nodes, checks their common row, and anchors their distributor.

use std::collections::BTreeMap;

use super::{
    CASE_LABEL_WIDTH, CASE_TIP_HEIGHT, CASE_WIDTH, LABEL_FONT, LINE_HEIGHT, Node, NodeId, Point,
    SELECT_SKEW, Scene,
};
use crate::text::{RichText, block_metrics, wrap_text};

const CASE_MIN_BODY_HEIGHT: i32 = 52;
const CASE_LABEL_PADDING_Y: i32 = 14;

/// A later case leaves the distributor from its right side.
pub(super) fn exit_anchor(node: &Node) -> Point {
    Point {
        x: node.x + (node.width - SELECT_SKEW) / 2,
        y: node.y,
    }
}

pub(super) fn case_dimensions(label: &RichText) -> (i32, i32, Vec<RichText>) {
    let lines = wrap_text(label, CASE_LABEL_WIDTH, LABEL_FONT);
    let body_height = CASE_MIN_BODY_HEIGHT
        .max(2 * CASE_LABEL_PADDING_Y + block_metrics(&lines, LABEL_FONT, LINE_HEIGHT).height);
    (CASE_WIDTH, body_height + CASE_TIP_HEIGHT, lines)
}

pub(super) fn verify(scene: &Scene) -> Option<String> {
    let mut rows = BTreeMap::new();
    for node in &scene.nodes {
        if let NodeId::Case { choice, .. } = node.id
            && *rows.entry(choice).or_insert(node.y) != node.y
        {
            return Some(format!(
                "choice {} draws its cases on different rows",
                choice + 1
            ));
        }
    }
    None
}
