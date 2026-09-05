//! Binds an action's outputs and continues along the selected order.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::{Bindings, Frame, block_body, input_bindings};
use kaalang_model::{Block, ExecutionPlan, Flow};

pub(super) fn guarded(block: &Block, bindings: &Bindings) -> TokenStream2 {
    let inputs = super::guarded::inputs(block, bindings);
    let body = block_body(&block.body);
    let values = block
        .outputs
        .iter()
        .enumerate()
        .map(|(index, output)| {
            Ident::new(
                &format!("__kaalang_value_{index}"),
                Span::mixed_site().located_at(output.span()),
            )
        })
        .collect::<Vec<_>>();
    let pattern = if let [value] = values.as_slice() {
        quote_spanned!(block.output_span=> #value)
    } else {
        quote_spanned!(block.output_span=> (#(#values,)*))
    };
    let outputs = block.outputs.iter().zip(values).map(|(output, value)| {
        let wire = bindings.wire(output);
        let gate = bindings.gate_value(output, &quote!(#value));
        quote!(#gate #wire = ::core::option::Option::Some(#value);)
    });
    quote! {
        let #pattern = { #inputs #body };
        #(#outputs)*
    }
}

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    next: &ExecutionPlan,
    scope: &[Frame<'_>],
) -> TokenStream2 {
    let continuation = super::flow(flow, next, bindings, scope);
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let output_wires = block
        .outputs
        .iter()
        .map(|output| bindings.wire(output))
        .collect::<Vec<_>>();
    // Zero outputs bind `()`, so Rust rejects a body that is not unit.
    let pattern = if output_wires.len() == 1 {
        let output = output_wires[0];
        quote_spanned!(block.output_span=> #output)
    } else {
        quote_spanned!(block.output_span=> (#(#output_wires,)*))
    };
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
