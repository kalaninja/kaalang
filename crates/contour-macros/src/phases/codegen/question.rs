//! Emits the `if` that carries a question's two branches.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::{Merged, capture_bindings};
use crate::body::block_body;
use crate::model::Graph;

pub(crate) fn emit(
    graph: &Graph,
    index: usize,
    branches: [TokenStream2; 2],
    merged: Option<Merged>,
) -> TokenStream2 {
    let block = &graph.blocks[index];
    let bindings = capture_bindings(&block.inputs, graph);
    let body = block_body(&block.body);
    let yes_wire = graph.wire(&block.outputs[0]);
    let no_wire = graph.wire(&block.outputs[1]);
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
    let merge = &graph.blocks[merged.index];
    let output_wire = graph.wire(&merge.outputs[0]);
    let continuation = merged.continuation;

    quote_spanned! {merge.span=>
        let #output_wire = #question;
        #continuation
    }
}
