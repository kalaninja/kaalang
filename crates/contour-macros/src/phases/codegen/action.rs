//! Binds an action's outputs and continues along its single path.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::capture_bindings;
use crate::body::block_body;
use crate::model::Graph;

pub(crate) fn emit(graph: &Graph, index: usize, continuation: TokenStream2) -> TokenStream2 {
    let block = &graph.blocks[index];
    let bindings = capture_bindings(&block.inputs, graph);
    let body = block_body(&block.body);
    let output_wires = block
        .outputs
        .iter()
        .map(|output| graph.wire(output))
        .collect::<Vec<_>>();
    let pattern = if block.tuple_output {
        quote_spanned!(block.output_span=> (#(#output_wires,)*))
    } else {
        let output = output_wires[0];
        quote_spanned!(block.output_span=> #output)
    };

    quote_spanned! {block.span=>
        let #pattern = {
            #bindings
            #body
        };
        #continuation
    }
}
