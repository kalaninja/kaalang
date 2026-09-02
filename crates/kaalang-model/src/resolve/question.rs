//! Resolves a question's output-consumption invariant.

use proc_macro2::Ident;
use syn::{Error, Result};

/// Every question output selects a branch and therefore needs a consumer.
pub(super) fn validate(unconsumed: &[&Ident]) -> Result<()> {
    let Some(output) = unconsumed.first() else {
        return Ok(());
    };

    Err(Error::new(
        output.span(),
        "every kaalang question output must have a consumer",
    ))
}
