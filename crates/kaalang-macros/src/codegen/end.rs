//! Emits the values one branch hands to the authored end block.

use kaalang_model::Block;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::Bindings;

pub(super) fn guarded(block: &Block, bindings: &Bindings) -> TokenStream2 {
    if block.inputs.is_empty() {
        return TokenStream2::new();
    }
    let values = block.inputs.iter().map(|input| {
        let wire = bindings.wire(&input.ident);
        quote_spanned!(input.ident.span()=> #wire.take().expect("a validated execution provides every end input"))
    }).collect::<Vec<_>>();
    if let [value] = values.as_slice() {
        quote!(#value)
    } else {
        quote!((#(#values,)*))
    }
}

pub(super) fn arrival(bindings: &Bindings, inputs: &[Ident]) -> TokenStream2 {
    if inputs.is_empty() {
        TokenStream2::new()
    } else {
        super::join::value(bindings, inputs)
    }
}
