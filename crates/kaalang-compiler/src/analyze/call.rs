//! Produces a call's outputs in source order.

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: State) {
    walk.sequence(block, state);
}
