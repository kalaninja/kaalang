//! Emits the `if` that carries a question's two branches.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::{Bindings, Frame, block_body, input_bindings, join};
use kaalang_model::{Block, Branch, Flow, Join};

pub(super) fn guarded(block: &Block, bindings: &Bindings) -> TokenStream2 {
    let inputs = super::guarded::inputs(block, bindings);
    let body = block_body(&block.body);
    let yes = bindings.wire(&block.outputs[0]);
    let no = bindings.wire(&block.outputs[1]);
    let yes_gate = bindings.gate_value(&block.outputs[0], &quote!(()));
    let no_gate = bindings.gate_value(&block.outputs[1], &quote!(()));
    quote_spanned! {block.span=>
        if { #inputs #body } {
            #yes_gate
            #yes = ::core::option::Option::Some(());
        } else {
            #no_gate
            #no = ::core::option::Option::Some(());
        }
    }
}

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: &[Branch; 2],
    converged: Option<&Join>,
    scope: &[Frame<'_>],
) -> TokenStream2 {
    let joins = converged.map_or(&[][..], std::slice::from_ref);
    let names = join::JoinRouting::new(index, joins.len(), branches, scope);
    let inner = Frame::nest(scope, index, names.as_ref());
    let yes_path = super::continuation(flow, &branches[0], bindings, &inner);
    let no_path = super::continuation(flow, &branches[1], bindings, &inner);
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

    join::emit(
        flow,
        bindings,
        question,
        joins,
        names.as_ref(),
        block.span,
        scope,
    )
}
