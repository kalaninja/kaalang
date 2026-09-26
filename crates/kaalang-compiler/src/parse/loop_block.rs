//! Parses a described cycle, its optional entry gate and its declared outputs.

use syn::{Attribute, Error, Expr, ExprBreak, ExprLoop, Result, parse_quote_spanned};

use super::{BlockSyntax, description, structural_block};
use crate::model::{Block, BlockKind, Input};

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
    // The header only gates entry; the body captures outer data itself.
    if let Some(extra) = syntax.inputs.get(1) {
        return Err(Error::new(
            extra.alias.span(),
            "a kaalang cycle header names at most one gate",
        ));
    }
    if let Some(gate) = syntax
        .inputs
        .first()
        .filter(|gate| gate.borrowed || gate.mutable)
    {
        return Err(Error::new(
            gate.alias.span(),
            "a kaalang cycle gate is a plain wire name, without `mut` or `&`",
        ));
    }
    Ok(syntax.into_block(Some(description), Vec::new()))
}

/// Appends one hidden boundary consumer per declared output to the body that
/// ends at `blocks.len()`, in declaration order (RFC 0006 §5.2). A consumer
/// captures its body-local output; the route that reaches it with that output
/// exports it after all of its remaining work.
pub(super) fn exports(blocks: &mut Vec<Block>, header: usize) -> Result<()> {
    let cycle = &blocks[header];
    for (position, output) in cycle.outputs.iter().enumerate() {
        if cycle.outputs[..position].contains(output) {
            return Err(Error::new(
                output.span(),
                "a kaalang cycle declares each output once",
            ));
        }
        let produced = blocks[header + 1..]
            .iter()
            .filter(|block| block.parent == Some(header))
            .any(|block| block.outputs.contains(output));
        if !produced {
            return Err(Error::new(
                output.span(),
                format!(
                    "a declared kaalang cycle output needs a producer named `{output}` in the cycle body"
                ),
            ));
        }
    }
    let consumers = (0..cycle.outputs.len())
        .map(|position| {
            let ident = cycle.outputs[position].clone();
            let alias = cycle.output_binding(position).ident.clone();
            let span = alias.span();
            let mut consumer = structural_block(
                BlockKind::Export,
                span,
                vec![Input {
                    borrowed: false,
                    mutable: false,
                    ident,
                    alias: alias.clone(),
                    derived: false,
                }],
            );
            consumer.body = parse_quote_spanned!(span=> #alias);
            consumer.parent = Some(header);
            consumer.export_target = Some(header);
            consumer
        })
        .collect::<Vec<_>>();
    blocks.extend(consumers);
    Ok(())
}

/// A cycle completes by producing a declared output, never by an authored break.
pub(super) fn structural_break(expression: &ExprBreak) -> Error {
    Error::new_spanned(
        expression,
        "a kaalang cycle has no structural `break`; it completes when a route reaches the end of its body with one of its declared outputs",
    )
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
