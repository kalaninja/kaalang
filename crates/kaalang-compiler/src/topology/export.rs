//! A cycle's boundary consumer ends at the result junction of its output.

use super::{CycleBoundary, Source};
use crate::model::Flow;

pub(super) fn result(flow: &Flow, boundaries: &[CycleBoundary], block: usize) -> Option<usize> {
    let target = flow.blocks[block]
        .export_target
        .expect("a boundary consumer has a cycle");
    let result = boundaries
        .iter()
        .find(|boundary| boundary.header == target)
        .and_then(|boundary| boundary.results.get(flow.exported_output(block)))
        .expect("a completed cycle has a result boundary");
    // A for cycle's completion leaves its for-end rather than a junction.
    match *result {
        Source::Junction(junction) => Some(junction),
        Source::Exit(_) => None,
    }
}
