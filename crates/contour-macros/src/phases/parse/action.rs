//! An action names its outputs and takes no companion attributes.

use syn::Result;

use super::{BlockSyntax, validate_description};
use crate::model::Block;

/// An action needs only a description; flow resolution checks its outputs.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    validate_description(syntax.kind_attribute, "Contour block")?;
    syntax.reject_companions()?;

    Ok(syntax.into_block())
}
