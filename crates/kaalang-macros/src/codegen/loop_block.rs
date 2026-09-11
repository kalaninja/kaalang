//! Captures entry inputs once, then emits a native loop and its continuation.

use proc_macro2::TokenStream;
use quote::quote_spanned;

use super::{Bindings, input_bindings, loop_label};
use kaalang_model::{ExecutionPlan, Flow};

pub(super) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    body: &ExecutionPlan,
    next: Option<&ExecutionPlan>,
) -> TokenStream {
    let block = &flow.blocks[index];
    let body = super::flow(flow, body, bindings);
    let label = loop_label(index, block.span);
    let captures = input_bindings(&block.inputs, bindings);
    let next = next.map(|next| super::flow(flow, next, bindings));
    quote_spanned! {block.span=>
        #[allow(unused_mut)]
        { #captures }
        #[allow(unused_labels, clippy::never_loop)]
        #label: loop { #body }
        #next
    }
}
