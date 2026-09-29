//! A cycle's continue uses its iteration tail for every explicit capture.

use super::Loop;
use crate::model::Flow;

pub(super) fn tail(flow: &Flow, loops: &[Loop], block: usize) -> usize {
    let target = flow.blocks[block]
        .parent
        .expect("a continue belongs to a cycle");
    loops
        .iter()
        .find(|loop_| loop_.header == target)
        .expect("a repeating cycle has an iteration tail")
        .tail
}
