//! Sequential blocks provide all their outputs together.

use super::{Condition, State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &mut State, runs: Condition) {
    for output in 0..walk.flow.blocks[block].outputs.len() {
        let provided = walk.produce(&mut state.available, block, output, runs);
        let rejected = walk.executions.conditions.minus(runs, provided);
        state.live = walk.executions.conditions.minus(state.live, rejected);
    }
}
