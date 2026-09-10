//! Emits native while loops with fresh condition aliases on every check.

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::Lifetime;

use super::{Bindings, block_body, input_bindings};
use kaalang_model::{ExecutionPlan, Flow};

fn label(index: usize, span: Span) -> Lifetime {
    Lifetime::new(
        &format!("'__kaalang_loop_{index}"),
        Span::mixed_site().located_at(span),
    )
}

pub(super) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    body: &ExecutionPlan,
    next: &ExecutionPlan,
) -> TokenStream {
    let block = &flow.blocks[index];
    let captures = input_bindings(&block.inputs, bindings);
    let condition = block_body(&block.body);
    let body = super::flow(flow, body, bindings);
    let next = super::flow(flow, next, bindings);
    let label = label(index, block.span);
    quote_spanned! {block.span=>
        // A valid body can finish the flow on every branch without repeating.
        #[allow(unused_labels, clippy::blocks_in_conditions, clippy::never_loop)]
        #label: while { #captures #condition } { #body }
        #next
    }
}

pub(super) fn repeat(flow: &Flow, index: usize) -> TokenStream {
    let span = flow.blocks[index].span;
    let label = label(index, span);
    quote_spanned!(span=> continue #label;)
}
