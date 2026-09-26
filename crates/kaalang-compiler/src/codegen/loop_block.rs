//! Runs the body in a native loop against the outer storage and captures its
//! result. The gate binds nothing (RFC 0006 §6.3).

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::Lifetime;

use super::{Bindings, loop_label};
use crate::{Branch, ExecutionPlan, Flow, Join};

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
    let looped = quote_spanned! {block.span=>
        #[allow(unused_labels, clippy::never_loop)]
        #label: loop { #body }
    };
    if block.branch_count() > 0 {
        // Several outputs each leave through their own labeled result block,
        // like the cases of a choice (RFC 0006 §6.3).
        let labels = (0..block.outputs.len())
            .map(|output| exit_label(index, output))
            .collect::<Vec<_>>();
        let dispatch = super::exits(flow, bindings, index, &labels, branches, looped);
        return super::join::emit(flow, bindings, index, joins, dispatch);
    }
    // A declared output binds the whole value; `(found,)` is `found`.
    let pattern = bindings.pattern(block.output_span, &block.outputs);
    let assignment = if block.outputs.len() == 1
        && bindings.hoisted.contains(&block.outputs[0])
        && !bindings.merged.contains(&block.outputs[0])
    {
        let wire = bindings.wire_at(&block.outputs[0]);
        quote_spanned!(block.span=> #wire = { #looped };)
    } else {
        quote_spanned!(block.span=> let #pattern = { #looped };)
    };
    let gates = block.outputs.iter().map(|output| bindings.gate(output));
    let next = branches
        .first()
        .map(|branch| super::flow(flow, &branch.plan, bindings));
    quote_spanned! {block.span=>
        #assignment
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
