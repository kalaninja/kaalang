//! Emits the `result` value one branch hands to the implicit end block.

use kaalang_model::Block;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote_spanned;

use super::Bindings;

pub(super) fn guarded(block: &Block, bindings: &Bindings) -> TokenStream2 {
    let [result] = block.inputs.as_slice() else {
        unreachable!("the end block captures exactly one wire")
    };
    let wire = bindings.wire(&result.ident);
    quote_spanned!(result.ident.span()=> #wire.take().expect("a validated execution provides the `result` wire"))
}

pub(super) fn arrival(bindings: &Bindings, result: &Ident) -> TokenStream2 {
    let wire = bindings.wire_at(result);
    quote_spanned!(result.span()=> #wire)
}
