//! The implicit end block: every flow finishes by producing its `end` wire.

use proc_macro2::{Ident, Span};
use syn::{Error, ItemFn, ReturnType, parse_quote, spanned::Spanned};

use crate::model::{Block, BlockKind, END_WIRE, Input};

/// Builds the end block no flow authors. It captures `end`, declares no
/// outputs, and has no body. Its span is the declared return type, or the flow
/// name when the flow returns unit, so a flow that never produces `end`
/// reports against the contract it fails to meet.
pub(super) fn block(function: &ItemFn) -> Block {
    let span = match &function.sig.output {
        ReturnType::Default => function.sig.ident.span(),
        output @ ReturnType::Type(..) => output.span(),
    };
    let end_wire = Ident::new(END_WIRE, span);

    Block {
        kind: BlockKind::End,
        description: None,
        question_branches: Vec::new(),
        case_descriptions: Vec::new(),
        outputs: Vec::new(),
        output_pattern: parse_quote!(()),
        output_span: span,
        inputs: vec![Input {
            borrowed: false,
            mutable: false,
            ident: end_wire.clone(),
            alias: end_wire,
        }],
        body: parse_quote!({}),
        span,
        parent: None,
        loop_end: None,
        loop_label: None,
        break_target: None,
    }
}

/// Reports the authored `#[end]` statement kaalang no longer has.
pub(super) fn authored(span: Span) -> Error {
    Error::new(
        span,
        "a kaalang flow has no end statement; the block that produces the `end` wire finishes it",
    )
}
