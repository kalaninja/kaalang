//! Resolves a structural break to an enclosing loop.

use syn::{Error, ExprBreak, Result, ext::IdentExt, spanned::Spanned};

use super::structural_block;
use crate::model::{Block, BlockKind, Input};

pub(super) fn parse(
    expression: &ExprBreak,
    inputs: Vec<Input>,
    parent: Option<usize>,
    blocks: &[Block],
) -> Result<Block> {
    if let Some(attribute) = expression.attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang break does not support attributes",
        ));
    }
    if let Some(value) = &expression.expr {
        return Err(Error::new_spanned(
            value,
            "a kaalang break does not return a value",
        ));
    }
    let target = std::iter::successors(parent, |&index| blocks[index].parent).find(|&index| {
        expression.label.as_ref().is_none_or(|label| {
            blocks[index]
                .loop_label
                .as_ref()
                .is_some_and(|found| found.ident.unraw() == label.ident.unraw())
        })
    });
    if let Some(index) = target {
        let mut block = structural_block(BlockKind::Break, expression.span(), inputs);
        block.break_target = Some(index);
        return Ok(block);
    }
    Err(Error::new_spanned(
        expression,
        match expression.label {
            Some(_) => "a kaalang break label must name an enclosing loop",
            None => "a kaalang break requires an enclosing loop",
        },
    ))
}
