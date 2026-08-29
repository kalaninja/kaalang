//! Emits Rust tokens from a validated execution plan.

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{FnArg, ItemFn, Pat};

use crate::model::{Branch, Capture, Graph, Merge, Plan};

mod action;
mod choice;
mod question;

/// Emits the Rust that runs one verified plan. The plan already proves every
/// Contour invariant, so nothing here reports an error to the author.
pub(crate) fn flow(graph: &Graph, plan: &Plan) -> TokenStream2 {
    match plan {
        Plan::Action { index, next } => action::emit(graph, *index, flow(graph, next)),
        Plan::Question {
            index,
            branches,
            merge,
        } => {
            let branches = [
                continuation(graph, &branches[0]),
                continuation(graph, &branches[1]),
            ];
            question::emit(graph, *index, branches, merged(graph, merge.as_ref()))
        }
        Plan::Choice {
            index,
            branches,
            merge,
        } => {
            let branches = branches
                .iter()
                .map(|branch| continuation(graph, branch))
                .collect::<Vec<_>>();
            choice::emit(graph, *index, &branches, merged(graph, merge.as_ref()))
        }
        Plan::Terminal { output } => {
            let wire = graph.wire(output);
            quote_spanned!(output.span()=> #wire)
        }
        Plan::Arrival { input } => {
            let wire = graph.wire(input);
            quote_spanned!(input.span()=> #wire)
        }
    }
}

/// A branch that ends the flow returns outright when its siblings continue past
/// a merge, because the enclosing expression then carries the merged value.
fn continuation(graph: &Graph, branch: &Branch) -> TokenStream2 {
    let tokens = flow(graph, &branch.plan);
    if branch.early_return {
        quote!(return { #tokens })
    } else {
        tokens
    }
}

/// Emits the continuation branches share once they converge at a merge.
fn merged(graph: &Graph, merge: Option<&Merge>) -> Option<Merged> {
    merge.map(|merge| Merged {
        index: merge.index,
        continuation: flow(graph, &merge.next),
    })
}

/// The continuation a branch point's branches share beyond their merge.
pub(crate) struct Merged {
    pub(crate) index: usize,
    pub(crate) continuation: TokenStream2,
}

/// Emits block-local aliases for explicitly captured wires.
pub(crate) fn capture_bindings(inputs: &[Capture], graph: &Graph) -> TokenStream2 {
    let bindings = inputs.iter().map(|input| {
        let ident = &input.ident;
        let wire = graph.wire(ident);
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
pub(crate) fn rename_source_bindings(function: &mut ItemFn, graph: &Graph) {
    for (argument, source) in function.sig.inputs.iter_mut().zip(&graph.sources) {
        let FnArg::Typed(argument) = argument else {
            unreachable!("source_wires rejects method receivers")
        };
        let Pat::Ident(parameter) = argument.pat.as_mut() else {
            unreachable!("source_wires accepts only simple parameter bindings")
        };
        parameter.ident = graph.wire(source).clone();
    }
}
