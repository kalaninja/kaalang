//! Emits the `if` that carries a question's two branches.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::{Merged, input_bindings};
use crate::body::block_body;
use crate::model::Flow;

pub(crate) fn emit(
    flow: &Flow,
    index: usize,
    branches: [TokenStream2; 2],
    merged: Option<Merged>,
) -> TokenStream2 {
    let block = &flow.blocks[index];
    let bindings = input_bindings(&block.inputs, flow);
    let body = block_body(&block.body);
    let yes_wire = flow.wire(&block.outputs[0]);
    let no_wire = flow.wire(&block.outputs[1]);
    let [yes_path, no_path] = branches;
    let question = quote_spanned! {block.span=>
        if {
            #bindings
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
    let output_wire = flow.wire(&merge.outputs[0]);
    let continuation = merged.continuation;

    quote_spanned! {merge.span=>
        let #output_wire = #question;
        #continuation
    }
}
