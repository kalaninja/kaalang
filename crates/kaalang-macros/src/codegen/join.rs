//! Binds alternative producer values and emits their shared continuation.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::Lifetime;

use super::Bindings;
use kaalang_model::{Flow, Join, JoinTarget};

/// A verified yield names its destination directly, including enclosing joins.
fn label(target: JoinTarget) -> Lifetime {
    Lifetime::new(
        &format!("'__kaalang_join_{}_{}", target.block, target.join),
        Span::mixed_site(),
    )
}

fn value(bindings: &Bindings, wires: &[Ident]) -> TokenStream2 {
    let wires = wires
        .iter()
        .map(|wire| bindings.wire_at(wire))
        .collect::<Vec<_>>();
    super::tuple(Span::call_site(), &wires)
}

/// Leaves the branch scope and hands its wires straight to the target binding.
pub(super) fn yield_to(bindings: &Bindings, wires: &[Ident], target: JoinTarget) -> TokenStream2 {
    let label = label(target);
    let value = value(bindings, wires);
    quote!(break #label #value)
}

/// Wraps the dispatch in its joins, innermost first. A yield to a later or
/// enclosing label skips the intervening continuations. Every continuation
/// either yields again or returns the flow result, so disjoint groups cannot
/// fall through into each other. Only the yielded wires leave a branch scope;
/// its other local values drop before the shared continuation begins.
pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    block: usize,
    joins: &[Join],
    mut dispatch: TokenStream2,
) -> TokenStream2 {
    let span = Span::mixed_site().located_at(flow.blocks[block].span);
    for (index, join) in joins.iter().enumerate() {
        let label = label(JoinTarget { block, join: index });
        let pattern = value(bindings, &join.wires);
        let continuation = super::flow(flow, &join.next, bindings);
        // A match arm supplies a coercion context: a bare labeled initializer
        // otherwise fixes its type from the first break (e.g. an array reference
        // before a slice). This single unit arm does not dispatch at runtime.
        dispatch = quote_spanned! {span=>
            let #pattern = match () {
                () => #label: { #dispatch },
            };
            #continuation
        };
    }
    dispatch
}
