//! Parses a described, self-contained cycle interface.

use syn::{Attribute, Error, Expr, ExprLoop, Result};

use super::{BlockSyntax, description};
use crate::model::Block;

pub(super) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    if syntax
        .closure
        .is_some_and(|closure| !matches!(closure.body.as_ref(), Expr::Block(_)))
    {
        return Err(Error::new_spanned(
            &syntax.body,
            "a kaalang cycle body must use braces",
        ));
    }
    let description = description(syntax.kind_attribute, "kaalang cycle")?;
    syntax.reject_companions()?;
    Ok(syntax.into_block(Some(description), Vec::new()))
}

pub(super) fn legacy(expression: &ExprLoop) -> Error {
    Error::new_spanned(
        expression,
        "structural kaalang loops are no longer supported; use a `#[cycle(\"description\")]` block",
    )
}

pub(super) fn legacy_attribute(attribute: &Attribute) -> Error {
    Error::new_spanned(
        attribute,
        "the legacy `#[loop]` attribute is no longer supported; use `#[cycle(\"description\")]`",
    )
}
