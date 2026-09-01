//! Emits the values one path hands to the authored End block.

use proc_macro2::{Ident, TokenStream as TokenStream2};

use super::Bindings;
use contour_model::{Flow, Plan};

pub(super) fn emit(flow: &Flow, bindings: &Bindings, body: &Plan) -> TokenStream2 {
    super::flow(flow, body, bindings)
}

pub(super) fn arrival(bindings: &Bindings, inputs: &[Ident]) -> TokenStream2 {
    if inputs.is_empty() {
        TokenStream2::new()
    } else {
        super::convergence::value(bindings, inputs)
    }
}
