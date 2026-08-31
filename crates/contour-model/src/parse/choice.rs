//! A choice pairs each declared case with one output and one match arm.

use proc_macro2::Ident;
use syn::{Attribute, Error, Expr, Result};

use super::{BlockSyntax, description};
use crate::choice::{choice_match, is_todo_body, is_todo_macro};
use crate::model::Block;

/// Reports a case attribute that appears before its choice declaration.
pub(super) fn case_before_choice(attribute: &Attribute) -> Error {
    Error::new_spanned(
        attribute,
        "a `#[case(\"description\")]` attribute must follow `#[choice(\"description\")]`",
    )
}

/// A choice owns the `#[case("...")]` attributes that describe its branches.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    let block_description = description(syntax.kind_attribute, "Contour block")?;
    let cases = syntax.accept_companions("case")?;
    let case_descriptions = cases
        .iter()
        .map(|case| description(case, "Contour choice case"))
        .collect::<Result<Vec<_>>>()?;
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

    Ok(syntax.into_block(Some(block_description), case_descriptions))
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
