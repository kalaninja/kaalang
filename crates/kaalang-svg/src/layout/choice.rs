//! Sizes case nodes, checks their common row, and anchors their distributor.

use std::collections::BTreeMap;

use super::{Node, NodeId, Point, Scene};

use super::{CASE_LABEL_WIDTH, CASE_TIP_HEIGHT, CASE_WIDTH, LABEL_FONT, LINE_HEIGHT, SELECT_SKEW};
use crate::text::{RichText, block_metrics, wrap_text};

pub(super) fn exit_anchor(node: &Node, branch: usize) -> Point {
    if branch == 0 {
        Point {
            x: node.x,
            y: node.y + node.height / 2,
        }
    } else {
        Point {
            x: node.x + (node.width - SELECT_SKEW) / 2,
            y: node.y,
        }
    }
}

pub(super) fn case_dimensions(label: &RichText) -> (i32, i32, Vec<RichText>) {
    let lines = wrap_text(label, CASE_LABEL_WIDTH, LABEL_FONT);
    let body_height = 52.max(28 + block_metrics(&lines, LABEL_FONT, LINE_HEIGHT).height);
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
