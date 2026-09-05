//! Schedules a dependency graph that has no structured branch-tree lowering.
//! Each logical wire has an optional value; a block runs once its inputs exist.

use std::collections::HashSet;

use kaalang_model::{Block, BlockKind, Flow};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};

use super::Bindings;

pub(super) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    inputs: &[Ident],
    blocks: &[usize],
) -> TokenStream {
    let mut declared = inputs.iter().collect::<HashSet<_>>();
    let bound = inputs.iter().map(|input| {
        let wire = bindings.wire(input);
        quote! {
            #[allow(unused_mut)]
            let mut #wire = ::core::option::Option::Some(#wire);
        }
    });
    let mut body = quote!(#(#bound)*);
    for &index in blocks {
        let block = &flow.blocks[index];
        // Declare a slot at its first producer, so a value created after an
        // await does not affect the future's auto traits before that await.
        for output in &block.outputs {
            if declared.insert(output) {
                let wire = bindings.wire(output);
                body.extend(quote!(let mut #wire = ::core::option::Option::None;));
            }
        }
        let computation = match block.kind {
            BlockKind::Action => super::action::guarded(block, bindings),
            BlockKind::Question => super::question::guarded(block, bindings),
            BlockKind::Choice => super::choice::guarded(block, bindings),
            BlockKind::End => unreachable!("the schedule contains computational blocks only"),
        };
        if block.inputs.is_empty() {
            body.extend(computation);
        } else {
            let ready = block.inputs.iter().map(|input| {
                let wire = bindings.wire(&input.ident);
                quote!(#wire.is_some())
            });
            body.extend(quote! { if #(#ready)&&* { #computation } });
        }
    }
    body.extend(super::end::guarded(
        flow.blocks.last().expect("a flow has end"),
        bindings,
    ));
    body
}

/// Captures only after the readiness check, using ordinary Rust borrowing and
/// moves. Taking a consuming input clears its slot for every later block.
pub(super) fn inputs(block: &Block, bindings: &Bindings) -> TokenStream {
    let inputs = block.inputs.iter().map(|input| {
        let alias = &input.alias;
        let wire = bindings.wire(&input.ident);
        let value = if input.borrowed {
            quote!(#wire.as_ref())
        } else {
            quote!(#wire.take())
        };
        quote_spanned! {alias.span()=>
            #[allow(unused_variables, clippy::let_unit_value)]
            let #alias = #value.expect("a scheduled block has every input");
        }
    });
    quote!(#(#inputs)*)
}
