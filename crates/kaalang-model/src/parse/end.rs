//! The implicit visual and planning boundary for explicit flow returns.

use proc_macro2::Span;
use syn::{Error, ItemFn, ReturnType, parse_quote, spanned::Spanned};

use crate::model::{Block, BlockKind};

/// Builds the completion boundary no flow author writes. Its span is the
/// declared return type, or the flow name when the flow returns unit.
pub(super) fn block(function: &ItemFn) -> Block {
    let span = match &function.sig.output {
        ReturnType::Default => function.sig.ident.span(),
        output @ ReturnType::Type(..) => output.span(),
    };
    Block {
        kind: BlockKind::End,
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
        break_target: None,
    }
}

/// Reports the authored `#[end]` statement kaalang no longer has.
pub(super) fn authored(span: Span) -> Error {
    Error::new(
        span,
        "a kaalang flow has no end statement; use a structural `return` to finish it",
    )
}
