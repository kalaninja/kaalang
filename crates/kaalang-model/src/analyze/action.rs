//! Produces an action's outputs and validates their capture coverage.

use proc_macro2::Ident;
use syn::Error;

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    let outputs = walk.flow.blocks[block].outputs.len();
    if (0..outputs).all(|output| walk.produce(&mut state, block, output)) {
        walk.visit(state);
    }
}

/// Every ordinary action output needs a consumer in some execution.
pub(super) fn uncaptured(output: &Ident) -> Error {
    Error::new(
        output.span(),
        "every kaalang action output must have a consumer",
    )
}
