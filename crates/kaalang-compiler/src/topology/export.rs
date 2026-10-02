//! A cycle's boundary consumer ends at the result junction of its output.

use super::CycleBoundary;
use crate::model::Flow;

pub(super) fn result(flow: &Flow, boundaries: &[CycleBoundary], block: usize) -> usize {
    let target = flow.blocks[block]
        .export_target
        .expect("a boundary consumer has a cycle");
    boundaries
        .iter()
        .find(|boundary| boundary.header == target)
        .and_then(|boundary| boundary.result_junction(flow.exported_output(block)))
        .expect("a completed cycle has a result boundary")
}
