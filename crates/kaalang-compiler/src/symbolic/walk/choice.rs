//! Choices select one output under the same condition as questions. A for
//! cycle out of items passes its body by, up to its export.

use super::{Condition, State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &mut State, runs: Condition) {
    super::question::visit(walk, block, state, runs);
    if let Some(header) = walk.flow.blocks[block]
        .parent
        .filter(|_| walk.flow.takes_next_item(block))
    {
        let selected = walk.executions.selected(block, 1);
        let exhausted = walk.executions.conditions.and(runs, selected);
        state.live = walk.executions.conditions.minus(state.live, exhausted);
        state
            .resume
            .push((walk.flow.exports(header).start, exhausted));
    }
}
