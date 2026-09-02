//! Binds path-exclusive values and emits their shared continuation.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::Bindings;
use contour_model::{Convergence, Flow};

pub(super) fn value(bindings: &Bindings, wires: &[Ident]) -> TokenStream2 {
    let wires = wires
        .iter()
        .map(|wire| bindings.wire_at(wire))
        .collect::<Vec<_>>();
    match wires.as_slice() {
        [] => quote!(()),
        [wire] => quote_spanned!(wire.span()=> #wire),
        wires => quote!((#(#wires,)*)),
    }
}

/// Wraps a branch expression in its optional implicit convergence.
pub(super) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    branch: TokenStream2,
    converged: Option<&Convergence>,
    span: Span,
) -> TokenStream2 {
    let Some(converged) = converged else {
        return branch;
    };
    let pattern = value(bindings, &converged.wires);
    let continuation = super::flow(flow, &converged.next, bindings);

    quote_spanned! {span=>
        let #pattern = #branch;
        #continuation
    }
}
