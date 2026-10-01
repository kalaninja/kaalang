//! Selections provide one outcome per participating history.

use super::{Condition, State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &mut State, runs: Condition) {
    walk.constrain(block, runs);
    for output in 0..walk.flow.blocks[block].outputs.len() {
        let selected = walk.executions.selected(block, output);
        let when = walk.executions.conditions.and(runs, selected);
        let provided = walk.produce(&mut state.available, block, output, when);
        let rejected = walk.executions.conditions.minus(when, provided);
        state.live = walk.executions.conditions.minus(state.live, rejected);
    }
}
