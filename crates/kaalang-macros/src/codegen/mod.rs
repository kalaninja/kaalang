//! Emits Rust tokens from a validated execution plan.

use std::collections::{HashMap, HashSet};

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Expr, FnArg, ItemFn, Pat, ext::IdentExt, token::Mut};

use kaalang_model::{ExecutionPlan, Flow, Input, SemanticModel};

mod action;
mod choice;
mod end;
mod join;
mod question;
mod while_loop;

/// Hygienic Rust bindings assigned locally for one lowering pass.
pub(crate) struct Bindings {
    wires: HashMap<Ident, Ident>,
    /// Wires whose internal bindings permit a mutable borrowing capture.
    mutable: HashSet<Ident>,
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
        let mutable = model
            .flow
            .blocks
            .iter()
            .flat_map(|block| &block.inputs)
            .filter(|input| input.borrowed && input.mutable)
            .map(|input| input.ident.clone())
            .collect();
        Self {
            wires,
            mutable,
            gates,
        }
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

    fn mutability(&self, name: &Ident) -> Option<Mut> {
        // This modifier belongs to generated storage, including producer
        // bindings moved through a merge before any mutable borrow happens.
        self.mutable.contains(name).then(|| Mut {
            span: Span::mixed_site().located_at(name.span()),
        })
    }

    pub(crate) fn is_mutably_captured(&self, name: &Ident) -> bool {
        self.mutable.contains(name)
    }

    pub(crate) fn pattern(&self, span: Span, names: &[Ident]) -> TokenStream2 {
        let bindings = names
            .iter()
            .map(|name| {
                let wire = self.wire_at(name);
                let mutable = self.mutability(name);
                quote!(#mutable #wire)
            })
            .collect::<Vec<_>>();
        tuple(span, &bindings)
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
        // Keep the helper item outside authored scopes: item names resolve at
        // the call site even under mixed-site hygiene, so a flow-level item
        // would be reachable from an authored body. The function marker carries
        // the inferred type without storing a value of it.
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

/// Emits one binding or value bare and several as a tuple.
pub(crate) fn tuple(span: Span, idents: &[impl ToTokens]) -> TokenStream2 {
    match idents {
        [] => quote_spanned!(span=> ()),
        [ident] => quote_spanned!(span=> #ident),
        idents => quote_spanned!(span=> (#(#idents,)*)),
    }
}

/// Emits the Rust that runs one verified plan. The plan already proves every
/// kaalang invariant, including the destination of every branch exit.
pub(crate) fn flow(flow: &Flow, plan: &ExecutionPlan, bindings: &Bindings) -> TokenStream2 {
    match plan {
        ExecutionPlan::While { index, body, next } => {
            while_loop::emit(flow, bindings, *index, body, next)
        }
        ExecutionPlan::Repeat { index } => while_loop::repeat(flow, *index),
        ExecutionPlan::Action { index, next } => action::emit(flow, bindings, *index, next),
        ExecutionPlan::Question {
            index,
            branches,
            join,
        } => question::emit(flow, bindings, *index, branches, join.as_ref()),
        ExecutionPlan::Choice {
            index,
            branches,
            joins,
        } => choice::emit(flow, bindings, *index, branches, joins),
        ExecutionPlan::End { body, .. } => {
            let gates = bindings.gate_declarations();
            let body = self::flow(flow, body, bindings);
            quote!(#gates #body)
        }
        ExecutionPlan::EndArrival { wire } => end::arrival(bindings, wire),
        ExecutionPlan::Yield { wires, join } => join::yield_to(bindings, wires, *join),
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
///
/// Each alias reads the wire at the capture's own span. Rust owns move and
/// borrow checking, so its diagnostics must name the block that took the value
/// rather than the one that produced it.
pub(crate) fn input_bindings(inputs: &[Input], bindings: &Bindings) -> TokenStream2 {
    let bindings = inputs.iter().map(|input| {
        // The alias keeps the authored spelling, so a wire named `r#type` binds.
        let alias = &input.alias;
        let wire = bindings.wire_at(&input.ident);
        let borrow = input.borrowed.then(|| quote_spanned!(alias.span()=> &));
        let mutable = input.mutable.then(|| quote_spanned!(alias.span()=> mut));
        let (binding_mut, borrow_mut) = if input.borrowed {
            (None, mutable)
        } else {
            (mutable, None)
        };
        quote_spanned!(alias.span()=>
            #[allow(unused_variables, clippy::let_unit_value)]
            let #binding_mut #alias = #borrow #borrow_mut #wire;
        )
    });

    quote!(#(#bindings)*)
}

/// Rewrites nested implementation parameters to their hygienic wire bindings.
pub(crate) fn rename_implementation_inputs(function: &mut ItemFn, bindings: &Bindings) {
    for argument in &mut function.sig.inputs {
        let FnArg::Typed(argument) = argument else {
            unreachable!("the parser rejects method receivers")
        };
        match argument.pat.as_mut() {
            Pat::Ident(parameter) => {
                let name = parameter.ident.unraw();
                parameter.mutability = parameter
                    .mutability
                    .as_ref()
                    .and(bindings.mutability(&name));
                parameter.ident = bindings.wire(&name).clone();
            }
            Pat::Wild(_) => {}
            _ => unreachable!("the parser accepts only simple bindings or wildcards"),
        }
    }
}
