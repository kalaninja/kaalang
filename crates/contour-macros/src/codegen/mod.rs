//! Emits Rust tokens from a validated execution plan.

use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{FnArg, ItemFn, Pat};

use contour_model::{Branch, Flow, Input, Merge, Plan};

mod action;
mod body;
mod choice;
mod question;

/// Hygienic Rust bindings assigned locally for one lowering pass.
pub(crate) struct Bindings {
    wires: HashMap<String, Ident>,
}

impl Bindings {
    pub(crate) fn new(flow: &Flow) -> Self {
        // Source spellings remain available for block-local input aliases.
        let wires = flow
            .sources
            .iter()
            .chain(flow.blocks.iter().flat_map(|block| &block.outputs))
            .enumerate()
            .map(|(index, wire)| {
                (
                    wire.to_string(),
                    Ident::new(
                        &format!("__contour_wire_{index}"),
                        Span::mixed_site().located_at(wire.span()),
                    ),
                )
            })
            .collect();
        Self { wires }
    }

    pub(crate) fn wire(&self, name: &Ident) -> &Ident {
        self.wires
            .get(&name.to_string())
            .expect("validated wires have lowering bindings")
    }
}

/// Emits the Rust that runs one verified plan. The plan already proves every
/// Contour invariant, so nothing here reports an error to the author.
pub(crate) fn flow(flow: &Flow, plan: &Plan, bindings: &Bindings) -> TokenStream2 {
    match plan {
        Plan::Action { index, next } => {
            action::emit(flow, bindings, *index, self::flow(flow, next, bindings))
        }
        Plan::Question {
            index,
            branches,
            merge,
        } => {
            let branches = [
                continuation(flow, &branches[0], bindings),
                continuation(flow, &branches[1], bindings),
            ];
            question::emit(
                flow,
                bindings,
                *index,
                branches,
                merged(flow, merge.as_ref(), bindings),
            )
        }
        Plan::Choice {
            index,
            branches,
            merge,
        } => {
            let branches = branches
                .iter()
                .map(|branch| continuation(flow, branch, bindings))
                .collect::<Vec<_>>();
            choice::emit(
                flow,
                bindings,
                *index,
                &branches,
                merged(flow, merge.as_ref(), bindings),
            )
        }
        Plan::Terminal { output } => {
            let wire = bindings.wire(output);
            quote_spanned!(output.span()=> #wire)
        }
        Plan::Arrival { input, .. } => {
            let wire = bindings.wire(input);
            quote_spanned!(input.span()=> #wire)
        }
    }
}

/// A branch that ends the flow returns outright when its siblings continue past
/// a merge, because the enclosing expression then carries the merged value.
fn continuation(flow: &Flow, branch: &Branch, bindings: &Bindings) -> TokenStream2 {
    let tokens = self::flow(flow, &branch.plan, bindings);
    if branch.early_return {
        quote!(return { #tokens })
    } else {
        tokens
    }
}

/// Emits the continuation branches share once they converge at a merge.
fn merged(flow: &Flow, merge: Option<&Merge>, bindings: &Bindings) -> Option<Merged> {
    merge.map(|merge| Merged {
        index: merge.index,
        continuation: self::flow(flow, &merge.next, bindings),
    })
}

/// The continuation a branch point's branches share beyond their merge.
pub(crate) struct Merged {
    pub(crate) index: usize,
    pub(crate) continuation: TokenStream2,
}

/// Emits block-local aliases for explicitly listed input wires.
pub(crate) fn input_bindings(inputs: &[Input], bindings: &Bindings) -> TokenStream2 {
    let bindings = inputs.iter().map(|input| {
        let ident = &input.ident;
        let wire = bindings.wire(ident);
        if input.borrowed {
            quote_spanned!(ident.span()=>
                #[allow(unused_variables)]
                let #ident = &#wire;
            )
        } else {
            quote_spanned!(ident.span()=>
                #[allow(unused_variables, clippy::let_unit_value)]
                let #ident = #wire;
            )
        }
    });

    quote!(#(#bindings)*)
}

/// Rewrites source parameters to their hygienic internal bindings.
pub(crate) fn rename_source_bindings(function: &mut ItemFn, flow: &Flow, bindings: &Bindings) {
    for (argument, source) in function.sig.inputs.iter_mut().zip(&flow.sources) {
        let FnArg::Typed(argument) = argument else {
            unreachable!("source_wires rejects method receivers")
        };
        let Pat::Ident(parameter) = argument.pat.as_mut() else {
            unreachable!("source_wires accepts only simple parameter bindings")
        };
        parameter.ident = bindings.wire(source).clone();
    }
}
