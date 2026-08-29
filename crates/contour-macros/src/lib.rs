//! Contour procedural macro entry point and compilation pipeline.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Error, ItemFn, Result, parse_macro_input};

mod phases;

// The data the phases hand over, and the helpers they share.
mod body;
mod model;

use crate::phases::{analyze, codegen, parse, resolve};

/// Parses and lowers an ordinary Rust function containing a Contour flow.
#[proc_macro_attribute]
pub fn contour(attributes: TokenStream, item: TokenStream) -> TokenStream {
    if !attributes.is_empty() {
        return Error::new(Span::call_site(), "#[contour] does not accept arguments")
            .into_compile_error()
            .into();
    }

    let mut function = parse_macro_input!(item as ItemFn);
    match expand(&mut function) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand(function: &mut ItemFn) -> Result<TokenStream2> {
    let parsed = parse::flow(function)?;
    let flow = resolve::flow(parsed)?;
    let plan = analyze::flow(&flow)?;
    let body = codegen::flow(&flow, &plan);

    codegen::rename_source_bindings(function, &flow);
    *function.block = syn::parse2(quote!({ #body }))?;

    Ok(quote! {
        #[allow(unreachable_code)]
        #function
    })
}
