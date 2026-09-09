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
    let body = codegen::flow(&model.flow, &model.execution_plan, &bindings);

    codegen::rename_flow_inputs(function, &bindings);
    *function.block = syn::parse2(quote!({ #body }))?;

    Ok(quote!(#function))
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use syn::{Expr, ItemFn, Stmt, parse_quote, visit::Visit};

    use super::expand;

    fn fixture(source: &str, name: &str) -> ItemFn {
        syn::parse_file(source)
            .expect("the behavior fixture is valid Rust")
            .items
            .into_iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == name => Some(function),
                _ => None,
            })
            .expect("the fixture declares its flow")
    }

    fn branching_matches(function: &ItemFn) -> usize {
        struct Count(usize);
        impl<'ast> Visit<'ast> for Count {
            fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
                self.0 += usize::from(expression.arms.len() > 1);
                syn::visit::visit_expr_match(self, expression);
            }
        }
        let mut count = Count(0);
        count.visit_item_fn(function);
        count.0
    }

    #[test]
    fn a_nested_early_return_needs_no_join_dispatch() {
        let mut function = fixture(
            include_str!(
                "../../kaalang/tests/wire/behavior/nested_branch_passes_a_question_join.rs"
            ),
            "nested_branch_passes_a_question_join",
        );
        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert!(expansion.contains("return "));
        assert_eq!(branching_matches(&function), 0);
        assert!(!expansion.contains("Result"));
        assert!(!expansion.contains("Option"));
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
        assert_eq!(branching_matches(&function), 0);
        assert!(!expansion.contains("Result"));
        assert!(!expansion.contains("Option"));
    }

    #[test]
    fn a_choice_dispatches_once_without_a_case_tag() {
        let mut function: ItemFn = parse_quote! {
            fn choose(input: Option<String>) -> usize {
                #[choice("Was text supplied?")]
                #[case("Text")]
                #[case("Absent")]
                |input| -> (text, absent) {
                    match input {
                        Some(value) => value,
                        None => 0u8,
                    }
                };

                #[action("Measure the text")]
                |text| -> selected { text.len() };

                #[action("Use the fallback")]
                |absent| -> selected { absent as usize };

                #[action("Finish")]
                |selected| -> result { __kaalang_shared_marker(selected) };
            }
        };
        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert_eq!(branching_matches(&function), 1);
        assert!(!expansion.contains("Result"));
        assert_eq!(
            expansion.matches("Option").count(),
            1,
            "only the authored input"
        );
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
        // The choice's authored match is the only dispatch. Labels add no
        // routing value or type item that could shadow an authored one.
        assert_eq!(branching_matches(&function), 1);
        assert!(!expansion.contains("enum "));
        assert!(!expansion.contains("struct "));
        assert!(!expansion.contains("Result"));
        assert!(!expansion.contains("Option"));
    }

    #[test]
    fn nested_partial_joins_emit_each_body_once_without_routing_values() {
        for (source, name) in [
            (
                include_str!("../../kaalang/tests/wire/behavior/const_partial_merge.rs"),
                "const_partial_merge",
            ),
            (
                include_str!(
                    "../../kaalang/tests/capture/behavior/const_borrowed_input_partial_merge.rs"
                ),
                "const_borrowed_input_partial_merge",
            ),
            (
                include_str!("../../kaalang/tests/wire/behavior/nested_partial_merges.rs"),
                "nested_partial_merges",
            ),
        ] {
            let mut function = fixture(source, name);
            let is_const = function.sig.constness.is_some();
            let mut markers = Vec::new();
            for (index, statement) in function.block.stmts.iter_mut().enumerate() {
                let Stmt::Expr(Expr::Closure(closure), _) = statement else {
                    panic!("each fixture statement declares a block");
                };
                let Expr::Block(body) = closure.body.as_mut() else {
                    panic!("each block has a braced body");
                };
                let marker = format_ident!("__kaalang_body_marker_{index}");
                if let [Stmt::Expr(Expr::Match(selection), _)] = body.block.stmts.as_mut_slice() {
                    // A choice still contains exactly its authored match.
                    let scrutinee = &selection.expr;
                    selection.expr = parse_quote!({ #marker(); #scrutinee });
                } else {
                    body.block.stmts.insert(0, parse_quote!(#marker();));
                }
                markers.push(format!("{marker} ("));
            }
            let expansion = expand(&mut function).expect("the flow expands").to_string();
            assert_eq!(branching_matches(&function), 1, "{name}");
            assert_eq!(expansion.contains("const fn"), is_const, "{name}");
            assert!(!expansion.contains("Result"), "{name}");
            assert!(!expansion.contains("Option"), "{name}");
            for marker in markers {
                assert_eq!(expansion.matches(&marker).count(), 1, "{name}: {marker}");
            }
        }
    }
}
