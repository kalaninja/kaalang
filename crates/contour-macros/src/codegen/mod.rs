//! Emits Rust tokens from a validated execution plan.

use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{Expr, FnArg, ItemFn, Pat, ext::IdentExt};

use contour_model::{Branch, Flow, Input, Plan};

mod action;
mod choice;
mod convergence;
mod end;
mod question;

/// Hygienic Rust bindings assigned locally for one lowering pass.
pub(crate) struct Bindings {
    wires: HashMap<Ident, Ident>,
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
                    wire.clone(),
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
            .get(name)
            .expect("validated wires have lowering bindings")
    }

    pub(crate) fn wire_at(&self, name: &Ident) -> Ident {
        let mut wire = self.wire(name).clone();
        wire.set_span(Span::mixed_site().located_at(name.span()));
        wire
    }
}

/// Emits the Rust that runs one verified plan. The plan already proves every
/// Contour invariant, so nothing here reports an error to the author.
pub(crate) fn flow(flow: &Flow, plan: &Plan, bindings: &Bindings) -> TokenStream2 {
    match plan {
        Plan::Action { index, next } => action::emit(flow, bindings, *index, next),
        Plan::Question {
            index,
            branches,
            convergence,
        } => question::emit(flow, bindings, *index, branches, convergence.as_ref()),
        Plan::Choice {
            index,
            branches,
            convergence,
        } => choice::emit(flow, bindings, *index, branches, convergence.as_ref()),
        Plan::End { body, .. } => self::flow(flow, body, bindings),
        Plan::EndArrival { inputs } => end::arrival(bindings, inputs),
        Plan::Yield { wires } => convergence::value(bindings, wires),
    }
}

/// A branch that ends the flow returns outright when its siblings enter a shared
/// continuation, because the enclosing expression then carries its value.
/// Branch plans must be lowered through this function, never through `flow`,
/// which would drop that early return.
fn continuation(flow: &Flow, branch: &Branch, bindings: &Bindings) -> TokenStream2 {
    let tokens = self::flow(flow, &branch.plan, bindings);
    if branch.early_return {
        quote!(return { #tokens })
    } else {
        tokens
    }
}

/// Splices the statements of a block body so lowering adds no extra braces.
pub(crate) fn block_body(body: &Expr) -> TokenStream2 {
    match body {
        Expr::Block(block) if block.attrs.is_empty() && block.label.is_none() => {
            let statements = &block.block.stmts;
            quote!(#(#statements)*)
        }
        body => quote!(#body),
    }
}

/// Emits block-local aliases for explicitly listed input wires.
pub(crate) fn input_bindings(inputs: &[Input], bindings: &Bindings) -> TokenStream2 {
    let bindings = inputs.iter().map(|input| {
        // The alias keeps the authored spelling, so a wire named `r#type` binds.
        let alias = &input.alias;
        let wire = bindings.wire(&input.ident);
        let borrow = input.borrowed.then(|| quote_spanned!(alias.span()=> &));
        quote_spanned!(alias.span()=>
            #[allow(unused_variables, clippy::let_unit_value)]
            let #alias = #borrow #wire;
        )
    });

    quote!(#(#bindings)*)
}

/// Rewrites source parameters to their hygienic internal bindings.
pub(crate) fn rename_source_bindings(function: &mut ItemFn, bindings: &Bindings) {
    for argument in &mut function.sig.inputs {
        let FnArg::Typed(argument) = argument else {
            unreachable!("source_wires rejects method receivers")
        };
        match argument.pat.as_mut() {
            Pat::Ident(parameter) => {
                parameter.ident = bindings.wire(&parameter.ident.unraw()).clone();
            }
            Pat::Wild(_) => {}
            _ => unreachable!("source_wires accepts only simple bindings or wildcards"),
        }
    }
}
