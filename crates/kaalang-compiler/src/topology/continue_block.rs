//! A cycle's continue uses its iteration tail for every explicit capture.

use super::Cycle;
use crate::model::Flow;

pub(super) fn tail(flow: &Flow, cycles: &[Cycle], block: usize) -> usize {
    let target = flow.blocks[block]
        .parent
        .expect("a continue belongs to a cycle");
    cycles
        .iter()
        .find(|cycle| cycle.header == target)
        .expect("a repeating cycle has an iteration tail")
        .tail
}
