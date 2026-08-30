//! Contour procedural macro entry point and compilation pipeline.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Error, ItemFn, Result, parse_macro_input};

mod codegen;

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
    let graph = contour_model::build(function)?;
    let bindings = codegen::Bindings::new(&graph.flow);
    let body = codegen::flow(&graph.flow, &graph.plan, &bindings);

    codegen::rename_source_bindings(function, &graph.flow, &bindings);
    *function.block = syn::parse2(quote!({ #body }))?;

    // ponytail: a branch that returns while its siblings continue past a merge
    // leaves the rest of its arm unreachable, so the allow covers the whole
    // function and also silences the lint inside authored bodies. Narrowing it
    // means attaching the allow to each generated `return` site.
    Ok(quote! {
        #[allow(unreachable_code)]
        #function
    })
}
