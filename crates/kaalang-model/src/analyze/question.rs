//! Explores a question's outputs and validates their capture coverage.

use proc_macro2::Ident;
use syn::Error;

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &State) {
    walk.branch(block, state);
}

/// Every ordinary question output needs a consumer in some execution.
pub(super) fn uncaptured(output: &Ident) -> Error {
    Error::new(
        output.span(),
        "every kaalang question output must have a consumer",
    )
}
