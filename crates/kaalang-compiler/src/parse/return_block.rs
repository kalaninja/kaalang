//! Parses a value-bearing transfer from the root flow.

use proc_macro2::Span;
use syn::{Error, Expr, ExprReturn, Result, parse_quote_spanned, spanned::Spanned};

use super::{captured, structural_block};
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
    let value = value(expression.expr.as_deref(), expression.span(), &inputs)?;
    let mut block = structural_block(BlockKind::Return, expression.span(), inputs);
    block.body = value;
    Ok(block)
}

/// Returns the one value a return carries, after checking that it consists
/// only of bindings introduced by its capture list.
fn value(value: Option<&Expr>, span: Span, inputs: &[Input]) -> Result<Expr> {
    let value = value
        .cloned()
        .unwrap_or_else(|| parse_quote_spanned!(span=> ()));
    validate(&value, inputs)?;
    Ok(value)
}

fn validate(value: &Expr, inputs: &[Input]) -> Result<()> {
    match value {
        Expr::Paren(parenthesized) if parenthesized.attrs.is_empty() => {
            validate(&parenthesized.expr, inputs)
        }
        Expr::Group(group) if group.attrs.is_empty() => validate(&group.expr, inputs),
        Expr::Tuple(tuple) if tuple.attrs.is_empty() => tuple
            .elems
            .iter()
            .try_for_each(|element| captured_input(element, inputs)),
        value => captured_input(value, inputs),
    }
}

fn captured_input(value: &Expr, inputs: &[Input]) -> Result<()> {
    let ident = match value {
        Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() => path.path.get_ident(),
        _ => None,
    };
    let Some(ident) = ident else {
        return Err(Error::new_spanned(
            value,
            "a kaalang return value must be a captured input, a tuple of captured inputs, or `()`",
        ));
    };
    if captured(inputs, ident) {
        Ok(())
    } else {
        Err(Error::new_spanned(
            ident,
            "a kaalang return value must name a captured input",
        ))
    }
}
