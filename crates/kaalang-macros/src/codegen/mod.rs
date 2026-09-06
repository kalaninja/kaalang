//! Emits Rust tokens from a validated execution plan.

use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{Expr, FnArg, ItemFn, Pat, ext::IdentExt};

use kaalang_model::{Branch, ExecutionPlan, Flow, Input, SemanticModel};

mod action;
mod choice;
mod end;
mod guarded;
mod join;
mod question;

pub(crate) use join::Frame;

/// Hygienic Rust bindings assigned locally for one lowering pass.
pub(crate) struct Bindings {
    wires: HashMap<Ident, Ident>,
    /// The type gate of each logical wire whose alternative producers no
    /// common binding unifies.
    gates: HashMap<Ident, Ident>,
}

impl Bindings {
    pub(crate) fn new(model: &SemanticModel) -> Self {
        // Flow-input spellings remain available for block-local input aliases.
        let wires = model
            .flow
            .flow_inputs
            .iter()
            .chain(model.flow.blocks.iter().flat_map(|block| &block.outputs))
            .enumerate()
            .map(|(index, wire)| {
                (
                    wire.clone(),
                    Ident::new(
                        &format!("__kaalang_wire_{index}"),
                        Span::mixed_site().located_at(wire.span()),
                    ),
                )
            })
            .collect();
        let ExecutionPlan::End { gates, .. } = &model.execution_plan else {
            unreachable!("the verified plan is rooted at end")
        };
        let gates = gates
            .iter()
            .enumerate()
            .map(|(index, wire)| {
                (
                    wire.clone(),
                    Ident::new(&format!("__kaalang_gate_{index}"), Span::mixed_site()),
                )
            })
            .collect();
        Self { wires, gates }
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

    /// Checks one just-bound producer occurrence against its wire's type gate,
    /// so alternative producers that no common binding unifies still share one
    /// Rust type. Empty for a wire without a gate.
    pub(crate) fn gate(&self, name: &Ident) -> TokenStream2 {
        let wire = self.wire_at(name);
        self.gate_value(name, &quote!(#wire))
    }

    pub(crate) fn gate_value(&self, name: &Ident, value: &TokenStream2) -> TokenStream2 {
        let Some(gate) = self.gates.get(name) else {
            return TokenStream2::new();
        };
        let check = Ident::new("__kaalang_same_type", Span::mixed_site());
        // Keep the helper item outside authored scopes. The function marker
        // carries the type without inheriting its auto traits across awaits.
        let declaration = quote! {
            const fn #check<T: ?Sized>(_: &::core::marker::PhantomData<fn(&T)>, _: &T) {}
        };
        quote_spanned!(name.span()=> {
            #declaration
            #check(&#gate, &#value);
        })
    }

    /// Declares one inferred gate per gated wire.
    fn gate_declarations(&self) -> TokenStream2 {
        let mut gates = self.gates.values().collect::<Vec<_>>();
        gates.sort();
        quote! {
            #(let #gates = ::core::marker::PhantomData;)*
        }
    }
}

/// Emits the Rust that runs one verified plan. The plan already proves every
/// kaalang invariant, so nothing here reports an error to the author. `scope`
/// lists the questions and choices enclosing the plan, outermost first.
pub(crate) fn flow(
    flow: &Flow,
    plan: &ExecutionPlan,
    bindings: &Bindings,
    scope: &[Frame<'_>],
) -> TokenStream2 {
    match plan {
        ExecutionPlan::Action { index, next } => action::emit(flow, bindings, *index, next, scope),
        ExecutionPlan::Guarded { inputs, blocks } => guarded::emit(flow, bindings, inputs, blocks),
        ExecutionPlan::Question {
            index,
            branches,
            join,
        } => question::emit(flow, bindings, *index, branches, join.as_ref(), scope),
        ExecutionPlan::Choice {
            index,
            branches,
            joins,
        } => choice::emit(flow, bindings, *index, branches, joins, scope),
        ExecutionPlan::End { body, .. } => {
            let gates = bindings.gate_declarations();
            let body = self::flow(flow, body, bindings, scope);
            quote!(#gates #body)
        }
        ExecutionPlan::EndArrival { result } => end::arrival(bindings, result),
        ExecutionPlan::Yield { wires, join } => join::yield_value(bindings, wires, *join, scope),
    }
}

/// A branch that ends the flow returns outright when a sibling yields to a
/// join, because the enclosing expression then carries the yielded value.
/// Branch plans must be lowered through this function, never through `flow`,
/// which would drop that early return.
fn continuation(
    flow: &Flow,
    branch: &Branch,
    bindings: &Bindings,
    scope: &[Frame<'_>],
) -> TokenStream2 {
    let tokens = self::flow(flow, &branch.plan, bindings, scope);
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

/// Rewrites flow-input parameters to their hygienic internal bindings.
pub(crate) fn rename_flow_inputs(function: &mut ItemFn, bindings: &Bindings) {
    for argument in &mut function.sig.inputs {
        let FnArg::Typed(argument) = argument else {
            unreachable!("the parser rejects method receivers")
        };
        match argument.pat.as_mut() {
            Pat::Ident(parameter) => {
                parameter.ident = bindings.wire(&parameter.ident.unraw()).clone();
            }
            Pat::Wild(_) => {}
            _ => unreachable!("the parser accepts only simple bindings or wildcards"),
        }
    }
}
