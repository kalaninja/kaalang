//! Leaves the target iteration and every nested iteration before continuing.

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    let target = walk.flow.blocks[block]
        .break_target
        .expect("a break has a target");
    state.available = state
        .loop_inputs
        .remove(&target)
        .expect("the target loop is active");
    // Loop indices follow lexical depth-first order, so later active entries
    // belong to iterations nested inside the target.
    state.loop_inputs.retain(|&index, _| index < target);
    walk.visit(
        walk.flow.blocks[target]
            .loop_end
            .expect("a loop owns a body"),
        state,
    );
}
