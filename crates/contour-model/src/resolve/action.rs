//! Resolves whether an action is a continuing or terminal block.

use proc_macro2::Ident;
use syn::{Error, Result};

/// An action is terminal exactly when its sole output has no consumer.
pub(super) fn terminal(outputs: &[Ident], unconsumed: &[&Ident]) -> Result<bool> {
    if unconsumed.is_empty() {
        return Ok(false);
    }
    if outputs.len() == 1 {
        return Ok(true);
    }

    Err(Error::new(
        unconsumed[0].span(),
        "a terminal Contour action must have exactly one output",
    ))
}
