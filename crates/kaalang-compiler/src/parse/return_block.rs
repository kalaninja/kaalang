//! Parses a value-bearing transfer from the root flow.

use syn::{Error, ExprReturn, Result, spanned::Spanned};

use super::{structural_block, transfer_value};
use crate::model::{Block, BlockKind, Input};

pub(super) fn parse(
    expression: &ExprReturn,
    inputs: Vec<Input>,
    parent: Option<usize>,
) -> Result<Block> {
    if let Some(attribute) = expression.attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang return does not support attributes",
        ));
    }
    if parent.is_some() {
        return Err(Error::new_spanned(
            expression,
            "a structural return is allowed only in the root kaalang sequence",
        ));
    }
    let value = transfer_value(
        expression.expr.as_deref(),
        expression.span(),
        &inputs,
        "return",
    )?;
    let mut block = structural_block(BlockKind::Return, expression.span(), inputs);
    block.body = value;
    Ok(block)
}
