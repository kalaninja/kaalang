//! An action names its outputs and takes no companion attributes.

use syn::Result;

use super::{BlockSyntax, description};
use crate::model::Block;

/// An action needs only a description; flow resolution checks its outputs.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    let description = description(syntax.kind_attribute, "kaalang block")?;
    syntax.reject_companions()?;

    Ok(syntax.into_block(Some(description), Vec::new()))
}
