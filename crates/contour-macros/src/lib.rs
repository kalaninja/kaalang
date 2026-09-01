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

    Ok(quote!(#function))
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    use super::expand;

    #[test]
    fn emits_a_shared_consumer_body_once() {
        let mut function: ItemFn = parse_quote! {
            fn choose(condition: bool) -> u32 {
                #[question("Choose a value")]
                |condition| -> (yes, no) { condition };

                #[action("Build the yes value")]
                |yes| -> selected { 1 };

                #[action("Build the no value")]
                |no| -> selected { 2 };

                #[action("Use the selected value")]
                |selected| -> result { __contour_shared_marker(selected) };

                #[end]
                |result| {};
            }
        };

        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert_eq!(expansion.matches("__contour_shared_marker").count(), 1);
    }
}
