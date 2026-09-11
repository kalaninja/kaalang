//! Parses a loop and its optional one-time entry captures.

use syn::{Error, ExprLoop, Result, spanned::Spanned};

use super::structural_block;
use crate::model::{Block, BlockKind, Input};

pub(super) fn parse(expression: &ExprLoop, inputs: Vec<Input>) -> Result<Block> {
    if let Some(attribute) = expression.attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang loop does not support attributes",
        ));
    }
    if let Some(label) = &expression.label
        && label.name.ident == "_"
    {
        return Err(Error::new_spanned(label, "`'_` is not a valid loop label"));
    }
    let mut block = structural_block(BlockKind::Loop, expression.span(), inputs);
    block.loop_label = expression.label.as_ref().map(|label| label.name.clone());
    Ok(block)
}
