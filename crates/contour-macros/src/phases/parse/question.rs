//! A question declares exactly two outputs: yes first, no second.

use syn::{Error, Result};

use super::{BlockSyntax, validate_description};
use crate::model::Block;

/// A question branches on a boolean, so it declares a yes and a no output.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    validate_description(syntax.kind_attribute, "Contour block")?;
    syntax.reject_companions()?;
    if !syntax.tuple_output || syntax.outputs.len() != 2 {
        return Err(Error::new_spanned(
            &syntax.closure.output,
            "a Contour question must declare exactly two outputs",
        ));
    }

    Ok(syntax.into_block())
}
