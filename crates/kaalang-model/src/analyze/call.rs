//! Produces a call's outputs and validates their capture coverage.

use proc_macro2::Ident;
use syn::Error;

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: State) {
    walk.sequence(block, state);
}

/// Every call output needs a consumer in some execution.
pub(super) fn uncaptured(output: &Ident) -> Error {
    Error::new(
        output.span(),
        "every kaalang call output must have a consumer",
    )
}
