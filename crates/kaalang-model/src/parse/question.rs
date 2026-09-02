//! A question declares exactly two outputs: yes first, no second.

use syn::{Error, Result};

use super::{BlockSyntax, description};
use crate::model::Block;

/// A question branches on a boolean, so it declares a yes and a no output.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    let description = description(syntax.kind_attribute, "kaalang block")?;
    syntax.reject_companions()?;
    if syntax.outputs.len() != 2 {
        return Err(Error::new_spanned(
            &syntax.closure.output,
            "a kaalang question must declare exactly two outputs",
        ));
    }

    Ok(syntax.into_block(Some(description), Vec::new()))
}
