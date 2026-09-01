//! Emits a choice's hygienic continuations and its authored dispatch.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::{Bindings, block_body, convergence, input_bindings, merge};
use contour_model::{Block, Branch, Convergence, Flow, Merge, choice_match, is_todo_body};

/// The identifiers one choice mints for itself. Every one is created at the
/// mixed site, so authored code can neither name them nor collide with them.
struct Names {
    /// One continuation macro per ordered case.
    continuations: Vec<Ident>,
    /// The unit type of the capability. It is neither `Copy` nor `Clone`, so
    /// a second use of the value is a move error rather than a second case.
    capability_type: Ident,
    /// The one value every continuation moves, so at most one case runs.
    capability: Ident,
    /// The binding that performs that move, typed so the error names the type.
    consumed: Ident,
}

impl Names {
    fn new(block: &Block, index: usize) -> Self {
        Self {
            continuations: (0..block.outputs.len())
                .map(|case| mint(&format!("__contour_continue_{index}_{case}")))
                .collect(),
            capability_type: mint(&format!("__ContourContinuationCapability{index}")),
            capability: mint(&format!("__contour_continuation_capability_{index}")),
            consumed: mint(&format!("__contour_consumed_capability_{index}")),
        }
    }
}

fn mint(name: &str) -> Ident {
    Ident::new(name, Span::mixed_site())
}

/// Defines one continuation macro per case. Definition-site hygiene keeps the
/// match bindings of one case out of every other case's downstream blocks.
fn definitions(
    block: &Block,
    bindings: &Bindings,
    branches: &[TokenStream2],
    names: &Names,
) -> Vec<TokenStream2> {
    let Names {
        continuations,
        capability_type,
        capability,
        consumed,
    } = names;

    block
        .outputs
        .iter()
        .zip(branches)
        .zip(continuations)
        .map(|((output, path), continuation)| {
            let output_wire = bindings.wire(output);
            quote! {
                macro_rules! #continuation {
                    ($value:expr) => {{
                        let #consumed: #capability_type = #capability;
                        let #output_wire = $value;
                        #path
                    }};
                }
            }
        })
        .collect()
}

/// Emits the match that selects a case and hands control to its continuation:
/// the authored `match` when the body has one, a case-indexed stub when the
/// body is still `todo!()`.
fn dispatch(block: &Block, bindings: &Bindings, names: &Names) -> TokenStream2 {
    let input_bindings = input_bindings(&block.inputs, bindings);
    let continuations = &names.continuations;

    if is_todo_body(&block.body) {
        let body = block_body(&block.body);
        let numbered = continuations[..continuations.len() - 1]
            .iter()
            .enumerate()
            .map(|(case, continuation)| quote!(#case => #continuation!(todo!()),));
        let last = continuations
            .last()
            .expect("a choice has at least two cases");
        return quote! {
            #[allow(clippy::diverging_sub_expression)]
            match {
                #input_bindings
                #body
            } {
                #(#numbered)*
                _ => #last!(todo!()),
            }
        };
    }

    let choice = choice_match(&block.body).expect("choice bodies are validated");
    let match_attrs = &choice.attrs;
    let scrutinee = &choice.expr;
    let arms = choice
        .arms
        .iter()
        .zip(continuations)
        .map(|(arm, continuation)| {
            let attrs = &arm.attrs;
            let pattern = &arm.pat;
            let value = &arm.body;
            quote! {
                #(#attrs)*
                #pattern => #continuation!(#value),
            }
        });

    quote! {
        {
            #input_bindings
            #(#match_attrs)*
            match #scrutinee {
                #(#arms)*
            }
        }
    }
}

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: &[Branch],
    merged: Option<&Merge>,
    converged: Option<&Convergence>,
) -> TokenStream2 {
    let branches = branches
        .iter()
        .map(|branch| super::continuation(flow, branch, bindings))
        .collect::<Vec<_>>();
    let block = &flow.blocks[index];
    let names = Names::new(block, index);
    let definitions = definitions(block, bindings, &branches, &names);
    let dispatch = dispatch(block, bindings, &names);
    let Names {
        capability_type,
        capability,
        ..
    } = &names;

    let tail = merge::emit(flow, bindings, dispatch, merged, Some(block.span));
    let tail = convergence::emit(flow, bindings, tail, converged, Some(block.span));

    quote_spanned! {block.span=>
        struct #capability_type;
        let #capability = #capability_type;
        #(#definitions)*
        #tail
    }
}
