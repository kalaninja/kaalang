//! Emits the `if` that carries a question's two branches.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::{Bindings, block_body, convergence, input_bindings};
use kaalang_model::{Branch, Convergence, Flow};

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: &[Branch; 2],
    converged: Option<&Convergence>,
) -> TokenStream2 {
    let yes_path = super::continuation(flow, &branches[0], bindings);
    let no_path = super::continuation(flow, &branches[1], bindings);
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let yes_wire = bindings.wire(&block.outputs[0]);
    let no_wire = bindings.wire(&block.outputs[1]);
    let question = quote_spanned! {block.span=>
        if {
            #input_bindings
            #body
        } {
            let #yes_wire = ();
            #yes_path
        } else {
            let #no_wire = ();
            #no_path
        }
    };

    convergence::emit(flow, bindings, question, converged, block.span)
}
