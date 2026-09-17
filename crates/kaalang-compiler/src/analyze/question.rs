//! Explores a question's branches.

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &State) {
    walk.branch(block, state);
}
