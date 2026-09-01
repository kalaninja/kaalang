//! Resolves a merge's output-consumption invariant.

use proc_macro2::Ident;
use syn::{Error, Result};

/// A merge is structural, so its output must feed a later block.
pub(super) fn validate(unconsumed: &[&Ident]) -> Result<()> {
    let Some(output) = unconsumed.first() else {
        return Ok(());
    };

    Err(Error::new(
        output.span(),
        "a Contour merge output must have a consumer",
    ))
}
