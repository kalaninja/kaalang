//! Resolves an action's output-consumption invariant.

use proc_macro2::Ident;
use syn::{Error, Result};

/// Every ordinary action output needs a consumer.
pub(super) fn validate(unconsumed: &[&Ident]) -> Result<()> {
    let Some(output) = unconsumed.first() else {
        return Ok(());
    };

    Err(Error::new(
        output.span(),
        "every kaalang action output must have a consumer",
    ))
}
