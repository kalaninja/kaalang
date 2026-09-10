//! Parses an unconditional loop, which has no condition block or attributes.

use syn::{Error, ExprLoop, Result, parse_quote};

use crate::model::{Block, BlockKind};

pub(super) fn parse(expression: &ExprLoop) -> Result<Block> {
    if let Some(attribute) = expression.attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang loop does not support attributes",
        ));
    }
    if let Some(label) = &expression.label {
        return Err(Error::new_spanned(
            label,
            "kaalang loops do not support labels",
        ));
    }
    let span = expression.loop_token.span;
    Ok(Block {
        kind: BlockKind::Loop,
        description: None,
        question_branches: Vec::new(),
        case_descriptions: Vec::new(),
        outputs: Vec::new(),
        output_pattern: parse_quote!(()),
        output_span: span,
        inputs: Vec::new(),
        body: parse_quote!({}),
        span,
        parent: None,
        loop_end: None,
    })
}
