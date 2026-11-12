//! A question positionally pairs its two outputs with one yes and one no branch.

use syn::{Attribute, Error, Meta, Result};

use super::{BlockSyntax, description};
use crate::model::{Block, QuestionBranch};

pub(super) fn answer_before_question(attribute: &Attribute) -> Error {
    Error::new_spanned(
        attribute,
        "a `#[kaa::yes]` or `#[kaa::no]` attribute must follow `#[kaa::question(\"description\")]`",
    )
}

/// A question branches on a boolean, so it declares a yes and a no output.
pub(crate) fn parse(syntax: BlockSyntax<'_>) -> Result<Block> {
    let description = description(syntax.kind_attribute, "kaalang block")?;
    let question_branches = answers(&syntax)?;
    syntax.require_inputs("question")?;
    if syntax.outputs.len() != 2 {
        return Err(Error::new(
            syntax.output_span,
            "a kaalang question must declare exactly two outputs",
        ));
    }

    let mut block = syntax.into_block(Some(description), Vec::new());
    block.question_branches = question_branches;
    Ok(block)
}

pub(super) fn answers(syntax: &BlockSyntax<'_>) -> Result<Vec<QuestionBranch>> {
    let attributes = syntax.accept_companions(&["yes", "no"])?;
    let question_branches = if attributes.is_empty() {
        vec![branch(true, None), branch(false, None)]
    } else {
        let branches = attributes
            .iter()
            .map(|attribute| parse_branch(attribute))
            .collect::<Result<Vec<_>>>()?;
        if branches.len() != 2 || branches[0].is_yes == branches[1].is_yes {
            return Err(Error::new_spanned(
                syntax.kind_attribute,
                "a kaalang question must declare exactly one `#[kaa::yes]` and one `#[kaa::no]` attribute, or neither",
            ));
        }
        branches
    };
    Ok(question_branches)
}

fn parse_branch(attribute: &Attribute) -> Result<QuestionBranch> {
    let description = match attribute.meta {
        Meta::Path(_) => None,
        Meta::List(_) => Some(description(attribute, "kaalang question branch")?),
        Meta::NameValue(_) => {
            return Err(Error::new_spanned(
                attribute,
                "a kaalang question branch attribute accepts only an optional string description",
            ));
        }
    };
    Ok(branch(super::is_annotation(attribute, "yes"), description))
}

fn branch(is_yes: bool, description: Option<String>) -> QuestionBranch {
    QuestionBranch {
        is_yes,
        description,
    }
}
