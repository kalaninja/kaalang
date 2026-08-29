//! A choice pairs each declared case with one output and one match arm.

use proc_macro2::Ident;
use syn::{Error, Expr, Result};

use super::{BlockSyntax, validate_description};
use crate::body::{choice_match, is_todo_body, is_todo_macro};
use crate::model::Block;

/// A choice owns the `#[case("...")]` attributes that describe its branches.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    validate_description(syntax.kind_attribute, "Contour block")?;
    if !syntax.tuple_output {
        return Err(Error::new_spanned(
            &syntax.closure.output,
            "a Contour choice must declare its outputs as a tuple",
        ));
    }

    let cases = syntax.accept_companions("case")?;
    for case in &cases {
        validate_description(case, "Contour choice case")?;
    }
    if cases.len() < 2 {
        return Err(Error::new_spanned(
            syntax.kind_attribute,
            "a Contour choice requires at least two `#[case(\"description\")]` attributes",
        ));
    }
    if cases.len() != syntax.outputs.len() {
        return Err(Error::new_spanned(
            &syntax.closure.output,
            "a Contour choice must declare exactly one output for each case",
        ));
    }

    validate_body(&syntax.body, &syntax.outputs)?;

    Ok(syntax.into_block())
}

/// Checks the restricted whole-body syntax accepted for choices.
fn validate_body(body: &Expr, outputs: &[Ident]) -> Result<()> {
    if is_todo_body(body) {
        return Ok(());
    }
    let Some(choice) = choice_match(body) else {
        return Err(Error::new_spanned(
            body,
            "a Contour choice body must contain exactly one `match` expression or `todo!()`",
        ));
    };
    if choice.arms.len() != outputs.len() {
        return Err(Error::new_spanned(
            choice,
            "a Contour choice match must contain exactly one arm for each output",
        ));
    }
    if let Some(arm) = choice
        .arms
        .iter()
        .find(|arm| is_todo_macro(&arm.body) || is_todo_body(&arm.body))
    {
        return Err(Error::new_spanned(
            &arm.body,
            "`todo!()` is supported only as the whole Contour choice body",
        ));
    }

    Ok(())
}
