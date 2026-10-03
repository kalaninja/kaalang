//! Runs the body in a native loop against the outer storage and captures its
//! result. A loop cycle's gate binds nothing; a for cycle's header captures
//! bind only while its iterator is built.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};
use syn::Lifetime;

use super::{Bindings, input_bindings, loop_label};
use crate::{Branch, ExecutionPlan, Flow, Iteration, Join};

pub(super) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    body: &ExecutionPlan,
    branches: &[Branch],
    joins: &[Join],
) -> TokenStream {
    let block = &flow.blocks[index];
    let body = super::flow(flow, body, bindings);
    let label = loop_label(index, block.span);
    let mut looped = quote_spanned! {block.span=>
        #[allow(unused_labels, clippy::never_loop)]
        #label: loop { #body }
    };
    if let Some(iteration) = &block.iteration {
        let iterator = Iteration::iterator();
        let captures = input_bindings(&block.inputs, bindings);
        let mut items = iteration.items.to_token_stream();
        if !captures.is_empty() {
            items = quote_spanned!(block.span=> { #captures #items });
        }
        looped = quote_spanned! {block.span=>
            let mut #iterator = ::core::iter::IntoIterator::into_iter(#items);
            #looped
        };
    }
    if block.branch_count() > 0 {
        // Several outputs each leave through their own labeled result block,
        // like the cases of a choice.
        let labels = (0..block.outputs.len())
            .map(|output| exit_label(index, output))
            .collect::<Vec<_>>();
        let dispatch = super::exits(flow, bindings, index, &labels, branches, looped);
        return super::join::emit(flow, bindings, index, joins, dispatch);
    }
    // A declared output binds the whole value; `(found,)` is `found`.
    let pattern = bindings.pattern(block.output_span, &block.outputs);
    let gates = block.outputs.iter().map(|output| bindings.gate(output));
    let next = branches
        .first()
        .map(|branch| super::flow(flow, branch, bindings));
    quote_spanned! {block.span=>
        let #pattern = { #looped };
        #(#gates)*
        #next
    }
}

/// The labeled result block one output of a cycle with several outputs leaves by.
pub(super) fn exit_label(index: usize, output: usize) -> Lifetime {
    Lifetime::new(
        &format!("'__kaalang_exit_{index}_{output}"),
        Span::mixed_site(),
    )
}
