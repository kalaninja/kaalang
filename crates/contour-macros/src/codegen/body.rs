//! Emits the authored Rust expressions used as block bodies.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Expr;

/// Splices the statements of a block body so lowering adds no extra braces.
pub(crate) fn block_body(body: &Expr) -> TokenStream2 {
    match body {
        Expr::Block(block) if block.attrs.is_empty() && block.label.is_none() => {
            let statements = &block.block.stmts;
            quote!(#(#statements)*)
        }
        body => quote!(#body),
    }
}
