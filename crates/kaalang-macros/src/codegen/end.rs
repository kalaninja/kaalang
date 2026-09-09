//! Returns the `result` value one branch hands to the implicit end block.

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote_spanned;

use super::Bindings;

pub(super) fn arrival(bindings: &Bindings, result: &Ident) -> TokenStream2 {
    let wire = bindings.wire_at(result);
    quote_spanned!(result.span()=> return #wire)
}
