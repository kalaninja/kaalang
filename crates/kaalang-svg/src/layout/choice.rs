//! Sizes case nodes and checks their common row.

use std::collections::BTreeMap;

use super::{NodeId, Scene};

use super::{
    CASE_LABEL_WIDTH, CASE_TIP_HEIGHT, CASE_WIDTH, LABEL_FONT, LINE_HEIGHT, text::wrap_text,
};

pub(super) fn case_dimensions(label: &str) -> (i32, i32, Vec<String>) {
    let lines = wrap_text(label, CASE_LABEL_WIDTH, LABEL_FONT);
    let body_height = 52.max(28 + lines.len() as i32 * LINE_HEIGHT);
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
