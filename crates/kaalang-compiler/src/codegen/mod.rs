//! Emits Rust tokens from a validated execution plan.

use std::collections::{HashMap, HashSet};

use proc_macro2::{Group, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Expr, FnArg, ItemFn, Lifetime, Pat, Result, Stmt, ext::IdentExt, token::Mut};

use crate::{Analysis, Block, BlockKind, Branch, ExecutionPlan, Flow, Input};

mod parameters;
#[cfg(test)]
mod tests;

mod choice;
mod cycle;
mod join;
mod question;
mod stage;

/// Hygienic Rust bindings assigned locally for one lowering pass.
pub(crate) struct Bindings {
    wires: HashMap<Ident, Ident>,
    /// Wires whose internal bindings permit a mutable borrowing capture.
    mutable: HashSet<Ident>,
    /// The type gate of each logical wire whose alternative producers no
    /// common binding unifies.
    gates: HashMap<Ident, Ident>,
    /// The final preparation region followed by the stage dispatcher.
    initial_dispatch: Option<(usize, TokenStream2)>,
    stages: usize,
    const_stages: bool,
}

impl Bindings {
    pub(crate) fn new(analysis: &Analysis) -> Self {
        let parameters = analysis
            .parameters
            .iter()
            .filter_map(|parameter| match parameter {
                FnArg::Typed(parameter) => match parameter.pat.as_ref() {
                    Pat::Ident(binding) => Some(binding.ident.unraw().to_string()),
                    _ => None,
                },
                FnArg::Receiver(_) => None,
            })
            .collect::<HashSet<_>>();
        // Flow-input spellings remain available for block-local input aliases.
        let wires = analysis
            .flow
            .flow_inputs
            .iter()
            .chain(analysis.flow.blocks.iter().flat_map(|block| &block.outputs))
            .enumerate()
            .map(|(index, wire)| {
                (
                    wire.clone(),
                    if wire == "self" {
                        wire.clone()
                    } else {
                        let mut name = format!("__kaalang_wire_{index}");
                        // Parameter bindings share generated hygiene. Reserve
                        // their spellings before moving them into wire storage.
                        while parameters.contains(&name) {
                            name.push('_');
                        }
                        Ident::new(&name, Span::mixed_site().located_at(wire.span()))
                    },
                )
            })
            .collect();
        let ExecutionPlan::End { gates, .. } = &analysis.execution_plan else {
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
        let mutable = analysis
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
            initial_dispatch: None,
            stages: analysis.stages.len(),
            const_stages: false,
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
        let value = &quote!(#wire);
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

/// Binds the block's outputs and emits its continuation.
fn in_place(flow: &Flow, bindings: &Bindings, index: usize, next: &ExecutionPlan) -> TokenStream2 {
    let continuation = self::flow(flow, next, bindings);
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let pattern = output_pattern(block, bindings);
    let value = quote_spanned!(block.span=> { #input_bindings #body });
    let gates = block.outputs.iter().map(|output| bindings.gate(output));

    quote_spanned! {block.span=>
        let #pattern = #value;
        #(#gates)*
        #continuation
    }
}

/// Captures a transfer's inputs and leaves through `exit`: `break` to a
/// cycle's loop or result block, `continue` to its loop, or `return` from the
/// root flow.
fn transfer(flow: &Flow, bindings: &Bindings, index: usize, exit: &TokenStream2) -> TokenStream2 {
    let block = &flow.blocks[index];
    let mut captures = input_bindings(&block.inputs, bindings);
    let mut value = transfer_value(&block.body);
    // A hidden export forwards a declared output, which may be named
    // `_like_this`: no author reads it here, so clippy must not report the read.
    if block.kind == BlockKind::Export {
        captures = hidden(captures);
        value = value.map(hidden);
    }
    quote_spanned! {block.span=>
        {
            #captures
            #exit #value;
        }
    }
}

/// Gives each identifier the expansion's hygiene at its own place.
fn hidden(tokens: TokenStream2) -> TokenStream2 {
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Group(group) => {
                let mut inner = Group::new(group.delimiter(), hidden(group.stream()));
                inner.set_span(group.span());
                inner.into()
            }
            TokenTree::Ident(mut ident) => {
                ident.set_span(Span::mixed_site().located_at(ident.span()));
                ident.into()
            }
            token => token,
        })
        .collect()
}

/// Emits Rust from a verified plan, including its resolved branch-exit targets.
pub(crate) fn flow(flow: &Flow, plan: &ExecutionPlan, bindings: &Bindings) -> TokenStream2 {
    let body = match plan {
        ExecutionPlan::Cycle {
            index,
            body,
            branches,
            joins,
        } => cycle::emit(flow, bindings, *index, body, branches, joins),
        ExecutionPlan::Export { index, target } => {
            let span = flow.blocks[*index].span;
            // One output is the loop's own value; several leave by their own
            // result blocks.
            let label = if flow.blocks[*target].branch_count() > 0 {
                cycle::exit_label(*target, flow.exported_output(*index))
            } else {
                loop_label(*target, span)
            };
            transfer(flow, bindings, *index, &quote_spanned!(span=> break #label))
        }
        ExecutionPlan::Return { index } => {
            if flow.blocks[*index].transition_target.is_some() {
                stage::transition(flow, bindings, *index)
            } else {
                let span = flow.blocks[*index].span;
                transfer(flow, bindings, *index, &quote_spanned!(span=> return))
            }
        }
        ExecutionPlan::Continue { index } => {
            let span = flow.blocks[*index].span;
            let target = flow.blocks[*index]
                .parent
                .expect("a continue belongs to a cycle");
            let label = loop_label(target, span);
            transfer(
                flow,
                bindings,
                *index,
                &quote_spanned!(span=> continue #label),
            )
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
    };
    stage::prepare(bindings, plan, body)
}

/// Binds each branch output from its own labeled block around `dispatch`,
/// innermost first, and follows it with that branch's continuation. Each
/// continuation leaves for a join or returns the flow result, so it cannot
/// fall through into the continuation of a different branch.
pub(super) fn exits(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    labels: &[Lifetime],
    branches: &[Branch],
    mut dispatch: TokenStream2,
) -> TokenStream2 {
    let block = &flow.blocks[index];
    for (output, branch) in branches.iter().enumerate() {
        let label = &labels[output];
        let wire = bindings.pattern(block.output_span, &block.outputs[output..=output]);
        let gate = bindings.gate(&block.outputs[output]);
        let path = self::flow(flow, branch, bindings);
        dispatch = quote! {
            let #wire = #label: { #dispatch };
            #gate
            #path
        };
    }
    dispatch
}

fn loop_label(index: usize, span: Span) -> Lifetime {
    Lifetime::new(
        &format!("'__kaalang_loop_{index}"),
        Span::mixed_site().located_at(span),
    )
}

/// Keeps a body's scope when a hoisted item can shadow a capture alias.
/// A statement macro can introduce such an item too.
pub(crate) fn block_body(body: &Expr) -> TokenStream2 {
    match body {
        Expr::Block(block) if block.attrs.is_empty() && block.label.is_none() => {
            let statements = &block.block.stmts;
            if statements
                .iter()
                .any(|stmt| matches!(stmt, Stmt::Item(_) | Stmt::Macro(_)))
            {
                // This extra block belongs to lowering, including its braces for Rust's lints.
                quote!({ #(#statements)* })
            } else {
                quote!(#(#statements)*)
            }
        }
        body => quote!(#body),
    }
}

/// Emits no operand for a unit transfer, matching native `break;` and `return;`.
fn transfer_value(body: &Expr) -> Option<TokenStream2> {
    (!matches!(body, Expr::Tuple(tuple) if tuple.elems.is_empty())).then(|| block_body(body))
}

/// Binds explicit captures at their own spans for Rust's move/borrow diagnostics.
pub(crate) fn input_bindings(inputs: &[Input], bindings: &Bindings) -> TokenStream2 {
    let bindings = inputs
        .iter()
        .filter(|input| input.ident != "self" && !input.derived)
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
            // The read keeps an alias used that only places its block, where an
            // `allow` would fail under `forbid`; rustc reports no unread `_name`,
            // and clippy would report this read of one.
            let read = (!alias.to_string().starts_with('_')).then(|| quote!(let _ = #alias;));
            // The binding takes the expansion's hygiene, where clippy reports no
            // binding of a unit control wire.
            quote_spanned!(Span::mixed_site().located_at(alias.span())=>
                let #binding_mut #alias = #borrow #borrow_mut #wire;
                #read
            )
        });

    quote!(#(#bindings)*)
}

/// Compiles a kaalang flow to Rust, preserving its signature and replacing its body.
///
/// # Errors
///
/// Returns the same errors as [`crate::build`].
pub fn expand(function: ItemFn) -> Result<ItemFn> {
    expand_in_scope(function, Span::mixed_site())
}

/// Expands as [`expand`] does, binding the parameters at `scope`, the span the
/// shell's signature gives their identifiers.
pub(crate) fn expand_in_scope(mut function: ItemFn, scope: Span) -> Result<ItemFn> {
    let analysis = crate::analyze(&function)?;
    if !analysis.stages.is_empty() {
        return stage::expand(function, &analysis, scope);
    }
    // Diagram realizability is required even when compilation discards the arrangement.
    let topology = crate::project(&analysis, false);
    crate::construct(&analysis, &topology)?;

    let bindings = Bindings::new(&analysis);
    let parameters = parameters::emit(&mut function, &bindings, scope);
    let body = flow(&analysis.flow, &analysis.execution_plan, &bindings);
    // Lower in place to preserve `Self`, enclosing generics, and receiver scope.
    *function.block = syn::parse2(quote!({
        {
            #parameters
            #body
        }
    }))?;

    Ok(function)
}
