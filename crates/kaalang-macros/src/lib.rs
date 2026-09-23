//! kaalang procedural macro entry point; compilation lives in `kaalang-compiler`.

use proc_macro::{Span, TokenStream};
use quote::ToTokens;
use syn::{Error, ItemFn, parse_macro_input};

/// Parses and lowers an ordinary Rust function containing a kaalang flow.
#[proc_macro_attribute]
pub fn kaalang(attributes: TokenStream, item: TokenStream) -> TokenStream {
    if !attributes.is_empty() {
        return Error::new(
            Span::call_site().into(),
            "#[kaalang] does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let function = parse_macro_input!(item as ItemFn);
    match kaalang_compiler::expand(function) {
        Ok(lowered) => lowered.into_token_stream().into(),
        Err(error) => error.into_compile_error().into(),
    }
}
