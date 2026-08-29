//! Emits Rust tokens from a validated execution plan.

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{FnArg, ItemFn, Pat};

use crate::model::{Branch, Flow, Input, Merge, Plan};

mod action;
mod choice;
mod question;

/// Emits the Rust that runs one verified plan. The plan already proves every
/// Contour invariant, so nothing here reports an error to the author.
pub(crate) fn flow(flow: &Flow, plan: &Plan) -> TokenStream2 {
    match plan {
        Plan::Action { index, next } => action::emit(flow, *index, self::flow(flow, next)),
        Plan::Question {
            index,
            branches,
            merge,
        } => {
            let branches = [
                continuation(flow, &branches[0]),
                continuation(flow, &branches[1]),
            ];
            question::emit(flow, *index, branches, merged(flow, merge.as_ref()))
        }
        Plan::Choice {
            index,
            branches,
            merge,
        } => {
            let branches = branches
                .iter()
                .map(|branch| continuation(flow, branch))
                .collect::<Vec<_>>();
            choice::emit(flow, *index, &branches, merged(flow, merge.as_ref()))
        }
        Plan::Terminal { output } => {
            let wire = flow.wire(output);
            quote_spanned!(output.span()=> #wire)
        }
        Plan::Arrival { input } => {
            let wire = flow.wire(input);
            quote_spanned!(input.span()=> #wire)
        }
    }
}

/// A branch that ends the flow returns outright when its siblings continue past
/// a merge, because the enclosing expression then carries the merged value.
fn continuation(flow: &Flow, branch: &Branch) -> TokenStream2 {
    let tokens = self::flow(flow, &branch.plan);
    if branch.early_return {
        quote!(return { #tokens })
    } else {
        tokens
    }
}

/// Emits the continuation branches share once they converge at a merge.
fn merged(flow: &Flow, merge: Option<&Merge>) -> Option<Merged> {
    merge.map(|merge| Merged {
        index: merge.index,
        continuation: self::flow(flow, &merge.next),
    })
}

/// The continuation a branch point's branches share beyond their merge.
pub(crate) struct Merged {
    pub(crate) index: usize,
    pub(crate) continuation: TokenStream2,
}

/// Emits block-local aliases for explicitly listed input wires.
pub(crate) fn input_bindings(inputs: &[Input], flow: &Flow) -> TokenStream2 {
    let bindings = inputs.iter().map(|input| {
        let ident = &input.ident;
        let wire = flow.wire(ident);
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
pub(crate) fn rename_source_bindings(function: &mut ItemFn, flow: &Flow) {
    for (argument, source) in function.sig.inputs.iter_mut().zip(&flow.sources) {
        let FnArg::Typed(argument) = argument else {
            unreachable!("source_wires rejects method receivers")
        };
        let Pat::Ident(parameter) = argument.pat.as_mut() else {
            unreachable!("source_wires accepts only simple parameter bindings")
        };
        parameter.ident = flow.wire(source).clone();
    }
}
