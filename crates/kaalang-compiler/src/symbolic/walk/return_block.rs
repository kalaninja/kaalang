//! Returns and transitions close the current history.

use super::{Condition, State, Walk};
use crate::ExecutionOutcome;

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &mut State, runs: Condition) {
    walk.finish(state, ExecutionOutcome::Return { block_index: block }, runs);
}
