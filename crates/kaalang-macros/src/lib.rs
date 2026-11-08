//! kaalang procedural macro entry point; compilation lives in `kaalang-compiler`.
//!
//! Use the attribute through the [`kaalang`](https://docs.rs/kaalang) crate,
//! which re-exports it.

use proc_macro::{Span, TokenStream};
use quote::ToTokens;
use syn::{Error, Item, parse_macro_input};

/// Turns a function written as a kaalang flow into ordinary Rust, after checking
/// that the flow has a diagram, or checks an annotated type and emits it without
/// its annotations. The [`kaalang`](https://docs.rs/kaalang) crate documentation
/// describes the syntax.
#[proc_macro_attribute]
pub fn kaa(attributes: TokenStream, item: TokenStream) -> TokenStream {
    let expanded = match parse_macro_input!(item as Item) {
        Item::Fn(_) if !attributes.is_empty() => Err(Error::new(
            Span::call_site().into(),
            "#[kaa] on a function does not accept arguments",
        )),
        Item::Fn(function) => kaalang_compiler::expand(function).map(ToTokens::into_token_stream),
        item => kaalang_compiler::expand_type(&attributes.into(), item),
    };
    match expanded {
        Ok(expanded) => expanded.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
