//! End captures ordered result wires and has no computational body.

use syn::{Error, Expr, Meta, Result, ReturnType};

use super::BlockSyntax;
use crate::model::Block;

/// End is structural: a bare attribute, bare inputs, no outputs, and no body.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    if !matches!(&syntax.kind_attribute.meta, Meta::Path(_)) {
        return Err(Error::new_spanned(
            syntax.kind_attribute,
            "`#[end]` does not accept arguments",
        ));
    }
    syntax.reject_companions()?;
    if let Some(borrowed) = syntax.inputs.iter().find(|input| input.borrowed) {
        return Err(Error::new(
            borrowed.ident.span(),
            "kaalang End inputs must be bare identifiers",
        ));
    }
    if !matches!(syntax.closure.output, ReturnType::Default) {
        return Err(Error::new_spanned(
            &syntax.closure.output,
            "a kaalang End block must not declare outputs",
        ));
    }
    validate_body(&syntax.body)?;

    Ok(syntax.into_block(None, Vec::new()))
}

fn validate_body(body: &Expr) -> Result<()> {
    let Expr::Block(block) = body else {
        return Err(Error::new_spanned(body, "a kaalang End body must be empty"));
    };
    if !block.attrs.is_empty() || block.label.is_some() || !block.block.stmts.is_empty() {
        return Err(Error::new_spanned(body, "a kaalang End body must be empty"));
    }

    Ok(())
}
