//! The implicit visual and planning boundary for explicit flow returns.

use proc_macro2::Span;
use syn::{Error, ItemFn, ReturnType, spanned::Spanned};

use crate::model::{Block, BlockKind};

/// Builds the completion boundary no flow author writes. Its span is the
/// declared return type, or the flow name when the flow returns unit.
pub(super) fn block(function: &ItemFn) -> Block {
    let span = match &function.sig.output {
        ReturnType::Default => function.sig.ident.span(),
        output @ ReturnType::Type(..) => output.span(),
    };
    super::structural_block(BlockKind::End, span, Vec::new())
}

/// Reports the authored `#[end]` statement kaalang no longer has.
pub(super) fn authored(span: Span) -> Error {
    Error::new(
        span,
        "a kaalang flow has no end statement; use a structural `return` to finish it",
    )
}
