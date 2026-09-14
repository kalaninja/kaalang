//! Binds persistent inputs once and captures the native loop's result.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use super::{Bindings, loop_label, output_pattern};
use kaalang_model::{ExecutionPlan, Flow};

fn input_bindings(flow: &Flow, bindings: &Bindings, index: usize) -> TokenStream {
    let inputs = flow.blocks[index].inputs.iter().map(|input| {
        let name = input
            .binding
            .as_ref()
            .expect("a cycle capture declares a local binding");
        let binding = bindings.wire_at(name);
        let wire = bindings.wire_at(&input.ident);
        let borrow = input
            .borrowed
            .then(|| quote_spanned!(input.alias.span()=> &));
        let mutable = input
            .mutable
            .then(|| quote_spanned!(input.alias.span()=> mut));
        let binding_mut = bindings.mutability(name);
        let borrow_mut = if input.borrowed { mutable } else { None };
        quote_spanned!(input.alias.span()=>
            #[allow(unused_mut, unused_variables, clippy::let_unit_value)]
            let #binding_mut #binding = #borrow #borrow_mut #wire;
        )
    });
    quote!(#(#inputs)*)
}

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
    let captures = input_bindings(flow, bindings, index);
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
