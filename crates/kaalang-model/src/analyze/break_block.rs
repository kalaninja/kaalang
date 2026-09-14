//! Completes the directly containing cycle and exposes its result interface.

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    let target = walk.flow.blocks[block]
        .break_target
        .expect("a break has a target");
    debug_assert_eq!(
        state.loops.last_key_value().map(|(&index, _)| index),
        Some(target)
    );
    let outside = state
        .loops
        .remove(&target)
        .expect("the target loop is active");
    state.available = outside.available;
    state.produced = outside.produced;
    let outputs = walk.flow.blocks[target].outputs.len();
    if (0..outputs).all(|output| walk.produce(&mut state, target, output)) {
        walk.visit(
            walk.flow.blocks[target]
                .loop_end
                .expect("a loop owns a body"),
            state,
        );
    }
}
