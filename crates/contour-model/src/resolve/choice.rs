//! Resolves a choice's output-consumption invariant.

use proc_macro2::Ident;
use syn::{Error, Result};

/// Every choice output selects a case and therefore needs a consumer.
pub(super) fn terminal(unconsumed: &[&Ident]) -> Result<bool> {
    let Some(output) = unconsumed.first() else {
        return Ok(false);
    };

    Err(Error::new(
        output.span(),
        "every Contour choice output must have a consumer",
    ))
}
