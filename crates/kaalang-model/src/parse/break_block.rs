//! Parses a value-bearing transfer from the current cycle.

use syn::{Error, ExprBreak, Result, spanned::Spanned};

use super::{structural_block, transfer_value};
use crate::model::{Block, BlockKind, Input};

pub(super) fn parse(
    expression: &ExprBreak,
    inputs: Vec<Input>,
    parent: Option<usize>,
) -> Result<Block> {
    if let Some(attribute) = expression.attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang break does not support attributes",
        ));
    }
    if let Some(label) = &expression.label {
        return Err(Error::new_spanned(
            label,
            "a kaalang break does not support labels; it completes the current cycle",
        ));
    }
    let Some(target) = parent else {
        return Err(Error::new_spanned(
            expression,
            "a kaalang break requires an enclosing cycle",
        ));
    };
    let value = transfer_value(
        expression.expr.as_deref(),
        expression.span(),
        &inputs,
        "break",
    )?;
    let mut block = structural_block(BlockKind::Break, expression.span(), inputs);
    block.body = value;
    block.break_target = Some(target);
    Ok(block)
}
