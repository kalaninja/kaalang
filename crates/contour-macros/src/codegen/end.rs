//! Emits the values one path hands to the authored End block.

use proc_macro2::{Ident, TokenStream as TokenStream2};

use super::Bindings;

pub(super) fn arrival(bindings: &Bindings, inputs: &[Ident]) -> TokenStream2 {
    if inputs.is_empty() {
        TokenStream2::new()
    } else {
        super::convergence::value(bindings, inputs)
    }
}
