//! Emits the `if` that carries a question's two branches.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote_spanned;

use super::{Bindings, block_body, input_bindings, join};
use kaalang_model::{Branch, Flow, Join};

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: &[Branch; 2],
    converged: Option<&Join>,
) -> TokenStream2 {
    let joins = converged.map_or(&[][..], std::slice::from_ref);
    let [yes_path, no_path] = branches
        .each_ref()
        .map(|branch| super::flow(flow, &branch.plan, bindings));
    let block = &flow.blocks[index];
    let inputs = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let yes_wire = bindings.wire(&block.outputs[0]);
    let no_wire = bindings.wire(&block.outputs[1]);
    let yes_gate = bindings.gate(&block.outputs[0]);
    let no_gate = bindings.gate(&block.outputs[1]);
    // The expansion context keeps lints such as `clippy::redundant_else` off a
    // branch that returns early; `located_at` keeps the authored position.
    let span = Span::mixed_site().located_at(block.span);
    let question = quote_spanned! {span=>
        if { #inputs #body } {
            let #yes_wire = ();
            #yes_gate
            #yes_path
        } else {
            let #no_wire = ();
            #no_gate
            #no_path
        }
    };

    join::emit(flow, bindings, index, joins, question)
}
