//! Ends one iteration at the transfer that repeats its cycle.

use super::{State, Walk};
use crate::model::ExecutionOutcome;

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: State) {
    let header = walk.flow.blocks[block]
        .parent
        .expect("a continue belongs to a cycle");
    debug_assert_eq!(
        state.loops.last_key_value().map(|(&index, _)| index),
        Some(header)
    );
    walk.record(state, ExecutionOutcome::Repeat { loop_index: header });
}
