//! Emits the `if` that carries a question's two branches.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote_spanned;

use super::{Bindings, block_body, input_bindings, join};
use crate::{Branch, Flow, Join};

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: &[Branch],
    joins: &[Join],
) -> TokenStream2 {
    let block = &flow.blocks[index];
    let yes = block
        .question_branches
        .iter()
        .position(|branch| branch.is_yes)
        .expect("a question declares one yes branch");
    let no = 1 - yes;
    let yes_path = super::flow(flow, &branches[yes].plan, bindings);
    let no_path = super::flow(flow, &branches[no].plan, bindings);
    let inputs = input_bindings(&block.inputs, bindings, false);
    let body = block_body(&block.body);
    let yes_wire = bindings.pattern(block.output_span, &block.outputs[yes..=yes]);
    let no_wire = bindings.pattern(block.output_span, &block.outputs[no..=no]);
    let yes_gate = bindings.gate(&block.outputs[yes]);
    let no_gate = bindings.gate(&block.outputs[no]);
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
