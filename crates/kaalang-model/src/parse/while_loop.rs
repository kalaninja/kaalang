//! A while uses a question's description and explicit condition captures.

use syn::{Error, Expr, ExprWhile, Result, parse_quote, spanned::Spanned};

use super::{
    BlockSyntax, block_closure, block_kind, description, question, reject_control_transfers,
};
use crate::model::{Block, BlockKind};

pub(super) fn parse(expression: &ExprWhile) -> Result<Block> {
    if let Some(label) = &expression.label {
        return Err(Error::new_spanned(
            label,
            "kaalang while loops do not support labels",
        ));
    }
    let (kind, attribute, companions) = block_kind(&expression.attrs, expression.span())?;
    if kind != BlockKind::Question {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang while requires `#[question(\"description\")]`",
        ));
    }
    let mut condition = expression.cond.as_ref();
    while let Expr::Paren(parenthesized) = condition {
        condition = &parenthesized.expr;
    }
    let Expr::Closure(closure) = condition else {
        return Err(Error::new_spanned(
            condition,
            "a kaalang while condition must have the form `(|inputs| condition)`",
        ));
    };
    if !closure.attrs.is_empty() {
        return Err(Error::new_spanned(
            closure,
            "a kaalang while condition carries its attributes on `while`",
        ));
    }
    let (inputs, body) = block_closure(closure)?;
    reject_control_transfers(&body)?;
    let syntax = BlockSyntax {
        kind: BlockKind::While,
        closure,
        kind_attribute: attribute,
        companions,
        inputs,
        outputs: Vec::new(),
        output_pattern: parse_quote!(()),
        output_span: expression.while_token.span,
        body,
    };
    syntax.require_inputs("question")?;
    let answers = question::answers(&syntax)?;
    let description = description(attribute, "kaalang block")?;
    let mut block = syntax.into_block(Some(description), Vec::new());
    block.question_branches = answers;
    Ok(block)
}
