//! Emits Rust tokens from a validated execution plan.

use std::collections::{HashMap, HashSet};

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Expr, Lifetime, Pat, token::Mut};

use kaalang_model::{Block, ExecutionPlan, Flow, Input, SemanticModel};

mod break_block;
mod choice;
mod join;
mod loop_block;
mod question;
mod return_block;

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
            .chain(model.flow.blocks.iter().flat_map(|block| {
                block
                    .inputs
                    .iter()
                    .filter_map(|input| input.binding.as_ref())
            }))
            .enumerate()
            .map(|(index, wire)| {
                (
                    wire.clone(),
                    if wire == "self" {
                        wire.clone()
                    } else {
                        Ident::new(
                            &format!("__kaalang_wire_{index}"),
                            Span::mixed_site().located_at(wire.span()),
                        )
                    },
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
        // The receiver is its own wire. Retyping its span in the macro's
        // hygiene context would stop `self` resolving to the authored one.
        wire.set_span(if wire == "self" {
            name.span()
        } else {
            Span::mixed_site().located_at(name.span())
        });
        wire
    }

    pub(crate) fn mutability(&self, name: &Ident) -> Option<Mut> {
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

/// Rewrites an authored output pattern to hygienic wire bindings.
pub(crate) fn output_pattern(block: &Block, bindings: &Bindings) -> TokenStream2 {
    let pattern = bindings.pattern(block.output_span, &block.outputs);
    if block.outputs.len() == 1 && matches!(block.output_pattern, Pat::Tuple(_)) {
        quote_spanned!(block.output_span=> (#pattern,))
    } else {
        pattern
    }
}

/// Emits the Rust that runs one verified plan. The plan already proves every
/// kaalang invariant, including the destination of every branch exit.
/// Binds a block's outputs from its own body and continues along the selected
/// order.
fn in_place(flow: &Flow, bindings: &Bindings, index: usize, next: &ExecutionPlan) -> TokenStream2 {
    let continuation = self::flow(flow, next, bindings);
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let pattern = output_pattern(block, bindings);
    let gates = block.outputs.iter().map(|output| bindings.gate(output));

    quote_spanned! {block.span=>
        let #pattern = {
            #input_bindings
            #body
        };
        #(#gates)*
        #continuation
    }
}

pub(crate) fn flow(flow: &Flow, plan: &ExecutionPlan, bindings: &Bindings) -> TokenStream2 {
    match plan {
        ExecutionPlan::Loop { index, body, next } => {
            loop_block::emit(flow, bindings, *index, body, next.as_deref())
        }
        ExecutionPlan::Break { index, target } => {
            break_block::emit(flow, bindings, *index, *target)
        }
        ExecutionPlan::Return { index } => return_block::emit(flow, bindings, *index),
        ExecutionPlan::Repeat { index } => {
            let label = loop_label(*index, flow.blocks[*index].span);
            quote_spanned!(flow.blocks[*index].span=> continue #label;)
        }
        // An action and a call both bind their outputs from their own body and
        // continue; only what the parser accepts as that body differs.
        ExecutionPlan::Action { index, next } | ExecutionPlan::Call { index, next } => {
            in_place(flow, bindings, *index, next)
        }
        ExecutionPlan::Question {
            index,
            branches,
            joins,
        } => question::emit(flow, bindings, *index, branches, joins),
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
        ExecutionPlan::Yield { wires, join } => join::yield_to(bindings, wires, *join),
    }
}

fn loop_label(index: usize, span: Span) -> Lifetime {
    Lifetime::new(
        &format!("'__kaalang_loop_{index}"),
        Span::mixed_site().located_at(span),
    )
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

/// Emits no operand for a unit transfer, matching native `break;` and `return;`.
pub(crate) fn transfer_value(body: &Expr) -> Option<TokenStream2> {
    (!matches!(body, Expr::Tuple(tuple) if tuple.elems.is_empty())).then(|| block_body(body))
}

/// Emits block-local aliases for explicitly listed input wires.
///
/// Each alias reads the wire at the capture's own span. Rust owns move and
/// borrow checking, so its diagnostics must name the block that took the value
/// rather than the one that produced it.
pub(crate) fn input_bindings(inputs: &[Input], bindings: &Bindings) -> TokenStream2 {
    let bindings = inputs
        .iter()
        .filter(|input| input.ident != "self")
        .map(|input| {
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
