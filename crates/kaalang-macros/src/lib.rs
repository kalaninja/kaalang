//! kaalang procedural macro entry point and compilation pipeline.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Error, ItemFn, Result, parse_macro_input};

mod codegen;

/// Parses and lowers an ordinary Rust function containing a kaalang flow.
#[proc_macro_attribute]
pub fn kaalang(attributes: TokenStream, item: TokenStream) -> TokenStream {
    if !attributes.is_empty() {
        return Error::new(Span::call_site(), "#[kaalang] does not accept arguments")
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
    let model = kaalang_model::build(function)?;
    let bindings = codegen::Bindings::new(&model);
    let body = codegen::flow(&model.flow, &model.execution_plan, &bindings, &[]);

    codegen::rename_flow_inputs(function, &bindings);
    *function.block = syn::parse2(quote!({ #body }))?;

    Ok(quote!(#function))
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    use super::expand;

    #[test]
    fn a_nested_early_return_needs_no_join_dispatch() {
        let file = syn::parse_file(include_str!(
            "../../kaalang/tests/wire/behavior/nested_branch_passes_a_question_join.rs"
        ))
        .expect("the behavior fixture is valid Rust");
        let mut function = file
            .items
            .into_iter()
            .find_map(|item| match item {
                syn::Item::Fn(function)
                    if function.sig.ident == "nested_branch_passes_a_question_join" =>
                {
                    Some(function)
                }
                _ => None,
            })
            .expect("the fixture declares its flow");
        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert!(expansion.contains("return "));
        assert!(!expansion.contains("match "));
        assert!(!expansion.contains(":: core :: result :: Result"));
    }

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
                |selected| -> result { __kaalang_shared_marker(selected) };
            }
        };

        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert_eq!(expansion.matches("__kaalang_shared_marker").count(), 1);
    }

    #[test]
    fn emits_every_body_once_across_two_joins_and_a_nested_question() {
        let mut function: ItemFn = parse_quote! {
            fn route(value: u8, condition: bool) -> u8 {
                #[choice("Which group?")]
                #[case("First of the left group")]
                #[case("Second of the left group")]
                #[case("Terminal")]
                #[case("First of the right group")]
                #[case("Second of the right group")]
                |value, &condition| -> (a, b, done, c, d) {
                    match value {
                        0 => (),
                        1 => (),
                        2 => (),
                        3 => (),
                        _ => (),
                    }
                };

                #[action("Build the left value from a")]
                |a| -> left { 1 };

                #[action("Build the left value from b")]
                |b| -> left { 2 };

                #[action("Produce the terminal result")]
                |done| -> result { __kaalang_terminal_marker(3) };

                #[question("Refine the right group")]
                |c, condition| -> (yes, no) { condition };

                #[action("Build the right value on yes")]
                |yes| -> right { 4 };

                #[action("Build the right value on no")]
                |no| -> right { 5 };

                #[action("Build the right value from d")]
                |d| -> right { 6 };

                #[action("Use the left value")]
                |left| -> result { __kaalang_left_marker(left) };

                #[action("Use the right value")]
                |right| -> result { __kaalang_right_marker(right) };
            }
        };

        let expansion = expand(&mut function).expect("the flow expands").to_string();
        for marker in [
            "__kaalang_terminal_marker",
            "__kaalang_left_marker",
            "__kaalang_right_marker",
        ] {
            assert_eq!(expansion.matches(marker).count(), 1, "{marker}");
        }
        // Case tags and join variants are standard `Result` values, so the
        // expansion declares no item that could shadow one in an authored body.
        assert!(!expansion.contains("enum "));
        assert!(!expansion.contains("struct "));
        assert!(expansion.contains(":: core :: result :: Result :: Ok"));
    }
}
