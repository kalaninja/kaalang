//! Emits the `if` that carries a question's two branches.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::{Bindings, Merged, body::block_body, input_bindings};
use contour_model::Flow;

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: [TokenStream2; 2],
    merged: Option<Merged>,
) -> TokenStream2 {
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let yes_wire = bindings.wire(&block.outputs[0]);
    let no_wire = bindings.wire(&block.outputs[1]);
    let [yes_path, no_path] = branches;
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

    let Some(merged) = merged else {
        return question;
    };
    let merge = &flow.blocks[merged.index];
    let output_wire = bindings.wire(&merge.outputs[0]);
    let continuation = merged.continuation;

    quote_spanned! {merge.span=>
        let #output_wire = #question;
        #continuation
    }
}
