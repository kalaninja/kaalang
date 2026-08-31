//! Binds a merge output and emits its shared continuation.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote_spanned;

use super::Bindings;
use contour_model::{Flow, Merge};

/// Wraps a branch expression in its optional merge. The span override retains
/// a branch kind's established diagnostic span when it differs from the merge.
pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    branch: TokenStream2,
    merged: Option<&Merge>,
    span: Option<Span>,
) -> TokenStream2 {
    let Some(merged) = merged else {
        return branch;
    };
    let block = &flow.blocks[merged.index];
    let output_wire = bindings.wire(&block.outputs[0]);
    let continuation = super::flow(flow, &merged.next, bindings);
    let span = span.unwrap_or(block.span);

    quote_spanned! {span=>
        let #output_wire = #branch;
        #continuation
    }
}
