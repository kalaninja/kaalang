//! kaalang procedural macro entry point; compilation lives in `kaalang-compiler`.
//!
//! Use the attribute through the [`kaalang`](https://docs.rs/kaalang) crate,
//! which re-exports it.

use proc_macro::{Ident, Span, TokenStream, TokenTree};
use syn::{Item, ItemEnum, parse_macro_input};

/// Turns a function written as a kaalang flow into ordinary Rust, after checking
/// that the flow has a diagram, or checks a datatype and emits it without
/// its annotations. The [`kaalang`](https://docs.rs/kaalang) crate documentation
/// describes the syntax.
#[proc_macro_attribute]
pub fn kaa(attributes: TokenStream, item: TokenStream) -> TokenStream {
    // A re-export or a renamed Cargo dependency still reaches our own macro.
    let defining_crate =
        || TokenStream::from(TokenTree::Ident(Ident::new("$crate", Span::mixed_site()))).into();
    kaalang_compiler::expand_attribute(
        &attributes.into(),
        parse_macro_input!(item as Item),
        defining_crate,
    )
    .unwrap_or_else(syn::Error::into_compile_error)
    .into()
}

/// A pending second invocation of `kaa` sees this attribute before rustc removes it.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __kaa_applied(_: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Records the parameters rustc kept in the local derive.
#[doc(hidden)]
#[proc_macro_derive(__KaalangParameters)]
pub fn parameter_choices(item: TokenStream) -> TokenStream {
    kaalang_compiler::parameter_choices(&parse_macro_input!(item as ItemEnum)).into()
}

/// The derive needs the configured variants, but the program needs no enum.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __kaa_discard(_: TokenStream, _: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[doc(hidden)]
#[proc_macro]
pub fn __kaa_configured_parameters(input: TokenStream) -> TokenStream {
    kaalang_compiler::configured_parameters(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
