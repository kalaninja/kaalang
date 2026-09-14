//! Binds an action's outputs and continues along the selected order.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::{Bindings, block_body, input_bindings, output_pattern};
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
    let pattern = output_pattern(block, bindings);
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
