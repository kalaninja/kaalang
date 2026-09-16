//! Completes one root execution at an authored return.

use super::{State, Walk};
use crate::model::ExecutionOutcome;

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: State) {
    walk.record(state, ExecutionOutcome::Return { block_index: block });
}
