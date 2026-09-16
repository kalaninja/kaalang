//! Binds persistent inputs once and captures the native loop's result.

use proc_macro2::TokenStream;
use quote::quote_spanned;

use super::{Bindings, input_bindings, loop_label, output_pattern};
use crate::{ExecutionPlan, Flow};

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
    let captures = input_bindings(&block.inputs, bindings, true);
    let pattern = output_pattern(block, bindings);
    let gates = block.outputs.iter().map(|output| bindings.gate(output));
    let next = next.map(|next| super::flow(flow, next, bindings));
    quote_spanned! {block.span=>
        let #pattern = {
            #captures
            #[allow(unused_labels, clippy::never_loop)]
            #label: loop { #body }
        };
        #(#gates)*
        #next
    }
}
