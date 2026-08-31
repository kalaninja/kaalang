//! A merge takes a bare attribute, alternative inputs, one output, no body.

use syn::{Error, Expr, Meta, Result};

use super::BlockSyntax;
use crate::model::Block;

/// A merge is structural: a bare attribute, alternative bare inputs, one
/// output, and an empty body.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    if !matches!(&syntax.kind_attribute.meta, Meta::Path(_)) {
        return Err(Error::new_spanned(
            syntax.kind_attribute,
            "`#[merge]` does not accept arguments",
        ));
    }
    syntax.reject_companions()?;
    if syntax.inputs.len() < 2 {
        return Err(Error::new_spanned(
            syntax.closure,
            "a Contour merge requires at least two alternative inputs",
        ));
    }
    if let Some(borrowed) = syntax.inputs.iter().find(|input| input.borrowed) {
        return Err(Error::new(
            borrowed.ident.span(),
            "Contour merge inputs must be bare identifiers",
        ));
    }
    if syntax.outputs.len() != 1 {
        return Err(Error::new_spanned(
            &syntax.closure.output,
            "a Contour merge must declare exactly one output identifier",
        ));
    }

    validate_body(&syntax.body)?;

    Ok(syntax.into_block(None, Vec::new()))
}

/// Ensures a structural merge has an empty body.
fn validate_body(body: &Expr) -> Result<()> {
    let Expr::Block(block) = body else {
        return Err(Error::new_spanned(
            body,
            "a Contour merge body must be empty",
        ));
    };
    if !block.attrs.is_empty() || block.label.is_some() || !block.block.stmts.is_empty() {
        return Err(Error::new_spanned(
            body,
            "a Contour merge body must be empty",
        ));
    }

    Ok(())
}
