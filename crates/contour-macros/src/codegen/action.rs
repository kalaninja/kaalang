//! Binds an action's outputs and continues along its single path.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::{Bindings, block_body, input_bindings};
use contour_model::{Flow, Plan};

pub(crate) fn emit(flow: &Flow, bindings: &Bindings, index: usize, next: &Plan) -> TokenStream2 {
    let continuation = super::flow(flow, next, bindings);
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let output_wires = block
        .outputs
        .iter()
        .map(|output| bindings.wire(output))
        .collect::<Vec<_>>();
    let pattern = if output_wires.len() == 1 {
        let output = output_wires[0];
        quote_spanned!(block.output_span=> #output)
    } else {
        quote_spanned!(block.output_span=> (#(#output_wires,)*))
    };

    quote_spanned! {block.span=>
        let #pattern = {
            #input_bindings
            #body
        };
        #continuation
    }
}
