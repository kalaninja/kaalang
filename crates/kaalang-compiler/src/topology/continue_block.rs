//! A cycle's continue uses its iteration tail for every explicit capture.

use super::{Cycle, CycleBoundary};
use crate::model::Flow;

/// The tail of the continue's cycle, or the rail of a for cycle whose
/// iteration endings meet before its for-end. Coalescing later folds a sole
/// ending into the for-end.
pub(super) fn tail(
    flow: &Flow,
    cycles: &[Cycle],
    boundaries: &[CycleBoundary],
    block: usize,
) -> Option<usize> {
    let target = flow.blocks[block]
        .parent
        .expect("a continue belongs to a cycle");
    cycles
        .iter()
        .find(|cycle| cycle.header == target)
        .map(|cycle| cycle.tail)
        .or_else(|| {
            boundaries
                .iter()
                .find(|boundary| boundary.header == target)?
                .caps?
                .rail
        })
}
