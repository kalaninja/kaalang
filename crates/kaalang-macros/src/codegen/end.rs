//! Returns the `end` value one branch hands to the implicit end block.

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote_spanned;

use super::Bindings;

pub(super) fn arrival(bindings: &Bindings, end_wire: &Ident) -> TokenStream2 {
    let wire = bindings.wire_at(end_wire);
    quote_spanned!(end_wire.span()=> return #wire)
}
