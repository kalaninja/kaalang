//! Enters one unconditional iteration; reaching its end records a repeat.

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    state.loop_inputs.insert(block, state.available.clone());
    walk.visit(block + 1, state);
}
