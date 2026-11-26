//! A choice pairs each declared case with one output and one match arm.

use proc_macro2::Ident;
use syn::{Attribute, Error, Expr, Result};

use super::{BlockSyntax, description};
use crate::{choice::choice_match, model::Block};

pub(super) fn case_before_choice(attribute: &Attribute) -> Error {
    Error::new_spanned(
        attribute,
        "a `#[kaa::case(\"description\")]` attribute must follow `#[kaa::choice(\"description\")]`",
    )
}

pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    let block_description = description(syntax.kind_attribute, "kaalang block")?;
    let cases = syntax.accept_companions(&["case"])?;
    let case_descriptions = cases
        .iter()
        .map(|case| description(case, "kaalang choice case"))
        .collect::<Result<Vec<_>>>()?;
    syntax.require_inputs("choice")?;
    if cases.len() < 2 {
        return Err(Error::new_spanned(
            syntax.kind_attribute,
            "a kaalang choice requires at least two `#[kaa::case(\"description\")]` attributes",
        ));
    }
    if cases.len() != syntax.outputs.len() {
        return Err(Error::new(
            syntax.output_span,
            "a kaalang choice must declare exactly one output for each case",
        ));
    }

    validate_body(&syntax.body, &syntax.outputs)?;

    Ok(syntax.into_block(Some(block_description), case_descriptions))
}

fn validate_body(body: &Expr, outputs: &[Ident]) -> Result<()> {
    let Some(choice) = choice_match(body) else {
        return Err(Error::new_spanned(
            body,
            "a kaalang choice body must contain exactly one `match` expression",
        ));
    };
    if choice.arms.len() != outputs.len() {
        return Err(Error::new_spanned(
            choice,
            "a kaalang choice match must contain exactly one arm for each output",
        ));
    }
    Ok(())
}
