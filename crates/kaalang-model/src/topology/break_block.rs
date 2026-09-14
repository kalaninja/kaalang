//! A cycle's break uses its result junction for every explicit capture.

use super::LoopBoundary;
use crate::model::Flow;

pub(super) fn result(flow: &Flow, boundaries: &[LoopBoundary], block: usize) -> usize {
    let target = flow.blocks[block]
        .break_target
        .expect("a break has a cycle target");
    boundaries
        .iter()
        .find(|boundary| boundary.header == target)
        .and_then(LoopBoundary::result_junction)
        .expect("a completed cycle has a result boundary")
}
