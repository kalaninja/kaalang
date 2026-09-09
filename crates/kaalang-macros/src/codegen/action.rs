//! Binds an action's outputs and continues along the selected order.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;
use syn::Pat;

use super::{Bindings, block_body, input_bindings};
use kaalang_model::{ExecutionPlan, Flow};

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    next: &ExecutionPlan,
) -> TokenStream2 {
    let continuation = super::flow(flow, next, bindings);
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let mut pattern = bindings.pattern(block.output_span, &block.outputs);
    // Structural joins carry one wire bare; the authored action may instead
    // destructure a singleton tuple to produce that wire.
    if block.outputs.len() == 1 && matches!(block.output_pattern, Pat::Tuple(_)) {
        pattern = quote_spanned!(block.output_span=> (#pattern,));
    }
    let gates = block.outputs.iter().map(|output| bindings.gate(output));

    quote_spanned! {block.span=>
        let #pattern = {
            #input_bindings
            #body
        };
        #(#gates)*
        #continuation
    }
}
