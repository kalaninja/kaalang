//! A call applies one function to its captured inputs and takes no companion
//! attributes.

use syn::{Error, Expr, ExprCall, Meta, Result, Stmt, ext::IdentExt};

use super::{BlockSyntax, description};
use crate::model::{Block, Input};

/// A call describes itself with the function it runs when no description is
/// written, so its attribute may be bare. Its body is that one application.
pub(crate) fn parse(mut syntax: BlockSyntax<'_>) -> Result<Block> {
    let description = match &syntax.kind_attribute.meta {
        Meta::Path(_) => None,
        _ => Some(description(syntax.kind_attribute, "kaalang call")?),
    };
    syntax.reject_companions()?;
    if let Some(input) = syntax.inputs.iter().find(|input| {
        // A borrowed capture spells its own mutability on the reference, which
        // the callee's parameter takes. An owned one reaches the argument
        // unchanged, so `mut` there could never do anything.
        input.mutable && !input.borrowed
    }) {
        return Err(Error::new_spanned(
            &input.alias,
            "a kaalang call passes its inputs on unchanged, so a `mut` capture has no effect",
        ));
    }
    // Braces delimit a sequence, and a call body is always one application.
    if let Some(closure) = syntax.closure
        && matches!(closure.body.as_ref(), Expr::Block(_))
    {
        return Err(braced(&closure.body));
    }
    syntax.body = Expr::Call(application(&syntax.body, &syntax.inputs)?);

    Ok(syntax.into_block(description, Vec::new()))
}

/// Unwraps the normalized body to the one application it must consist of, and
/// checks that it names a function and passes captured inputs.
///
/// The body keeps that application rather than the one lowering would build, so
/// its spans still cover what the author wrote and a presentation can read the
/// callee from it.
fn application(body: &Expr, inputs: &[Input]) -> Result<ExprCall> {
    let Expr::Block(block) = body else {
        unreachable!("a parsed block body is a block expression")
    };
    // The trailing semicolon does not change what an application evaluates to,
    // and a call without outputs reads better with it.
    let [Stmt::Expr(expression, _)] = block.block.stmts.as_slice() else {
        return Err(rejected(body));
    };
    let Expr::Call(call) = ungrouped(expression) else {
        return Err(rejected(expression));
    };
    if !call.attrs.is_empty() {
        return Err(braced(expression));
    }

    let Expr::Path(path) = ungrouped(&call.func) else {
        return Err(Error::new_spanned(
            &call.func,
            "a kaalang call must name the function it calls with a path",
        ));
    };
    if path.qself.is_none()
        && let Some(name) = path.path.get_ident()
        && captured(inputs, name)
    {
        return Err(Error::new_spanned(
            name,
            "a kaalang call cannot take the function it calls from a wire; an action applies a value already on a wire",
        ));
    }

    for argument in &call.args {
        let Expr::Path(path) = ungrouped(argument) else {
            return Err(argument_rejected(argument));
        };
        if !path.attrs.is_empty() || path.qself.is_some() {
            return Err(argument_rejected(argument));
        }
        let Some(name) = path.path.get_ident() else {
            return Err(argument_rejected(argument));
        };
        if !captured(inputs, name) {
            return Err(Error::new_spanned(
                name,
                "a kaalang call argument must name one of the block's captured inputs",
            ));
        }
    }

    Ok(call.clone())
}

/// Peels the invisible group a `macro_rules!` substitution arrives in.
///
/// Authored parentheses are left alone: unlike a transfer value, a call's path
/// is read back as the node's name, and `(math::difference)(left, right)` would
/// be a second spelling of one name.
fn ungrouped(mut expression: &Expr) -> &Expr {
    while let Expr::Group(group) = expression {
        expression = &group.expr;
    }
    expression
}

fn captured(inputs: &[Input], name: &proc_macro2::Ident) -> bool {
    inputs
        .iter()
        .any(|input| input.alias.unraw() == name.unraw())
}

/// A call body carries nothing around its application: no braces, and so no
/// attribute either, since rustfmt writes an attributed body with braces.
pub(super) fn braced(body: &Expr) -> Error {
    Error::new_spanned(
        body,
        "a kaalang call body is one application and takes no braces or attributes of its own; that work belongs in an action",
    )
}

fn rejected(body: &Expr) -> Error {
    Error::new_spanned(
        body,
        "a kaalang call body must apply a function path to captured inputs, as in `increment(value)`",
    )
}

fn argument_rejected(argument: &Expr) -> Error {
    Error::new_spanned(
        argument,
        "a kaalang call argument must be one of the block's captured inputs; that work belongs in an action",
    )
}
