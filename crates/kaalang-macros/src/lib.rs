//! kaalang procedural macro entry point and compilation pipeline.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use syn::{
    Error, FnArg, GenericParam, ItemFn, Pat, Result, Safety, Visibility, ext::IdentExt,
    parse_macro_input,
};

use kaalang_model::SemanticModel;

mod codegen;
#[cfg(test)]
mod performance;

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

    let implementation = implementation(function, &model, &bindings)?;
    let call = call(function, &implementation.sig.ident, &bindings);
    *function.block = syn::parse2(quote!({
        #implementation
        #[allow(clippy::used_underscore_binding)]
        #call
    }))?;

    Ok(quote!(#function))
}

/// Rewrites the authored function into the nested flow implementation.
fn implementation(
    function: &ItemFn,
    model: &SemanticModel,
    bindings: &codegen::Bindings,
) -> Result<ItemFn> {
    let body = codegen::flow(&model.flow, &model.execution_plan, bindings);

    let mut implementation = function.clone();
    implementation.attrs.clear();
    implementation.vis = Visibility::Inherited;
    implementation.sig.ident = syn::Ident::new("__kaalang_flow", Span::mixed_site());
    implementation.sig.inputs = implementation
        .sig
        .inputs
        .into_iter()
        .filter(|argument| !matches!(argument, FnArg::Typed(argument) if matches!(argument.pat.as_ref(), Pat::Wild(_))))
        .collect();
    codegen::rename_implementation_inputs(&mut implementation, bindings);
    *implementation.block = syn::parse2(quote!({ #body }))?;

    Ok(implementation)
}

/// Calls the implementation with the authored parameters and generics.
fn call(
    function: &ItemFn,
    implementation: &syn::Ident,
    bindings: &codegen::Bindings,
) -> TokenStream2 {
    let arguments = function.sig.inputs.iter().filter_map(|argument| {
        let FnArg::Typed(argument) = argument else {
            unreachable!("the parser rejects method receivers")
        };
        let Pat::Ident(parameter) = argument.pat.as_ref() else {
            return None;
        };
        let ident = &parameter.ident;
        Some(
            if parameter.mutability.is_some() && bindings.is_mutably_captured(&ident.unraw()) {
                quote!({ let _ = &mut #ident; #ident })
            } else {
                quote!(#ident)
            },
        )
    });
    let generic_arguments = function
        .sig
        .generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            GenericParam::Type(parameter) => Some(parameter.ident.to_token_stream()),
            GenericParam::Const(parameter) => Some(parameter.ident.to_token_stream()),
            GenericParam::Lifetime(_) => None,
        })
        .collect::<Vec<_>>();
    let generic_arguments =
        (!generic_arguments.is_empty()).then(|| quote!(::<#(#generic_arguments),*>));
    let call = quote!(#implementation #generic_arguments (#(#arguments),*));
    matches!(function.sig.safety, Safety::Unsafe(_))
        .then(|| quote!(unsafe { #call }))
        .unwrap_or(call)
}

#[cfg(test)]
mod tests {
    use quote::{ToTokens, format_ident};
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
    fn preserves_the_authored_function_signature() {
        let mut function: ItemFn = parse_quote! {
            pub const fn identity<T>(mut r#type: T, _: u8) -> T {
                |r#type| return r#type;
            }
        };
        let signature = function.sig.to_token_stream().to_string();

        expand(&mut function).expect("the flow expands");

        assert_eq!(function.sig.to_token_stream().to_string(), signature);
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
                let (yes, no) = |condition| { condition };

                #[action("Build the yes value")]
                let selected = |yes| { 1 };

                #[action("Build the no value")]
                let selected = |no| { 2 };

                #[action("Use the selected value")]
                let end = |selected| { __kaalang_shared_marker(selected) };

                |end| return end;
            }
        };

        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert_eq!(expansion.matches("__kaalang_shared_marker").count(), 1);
        assert_eq!(branching_matches(&function), 0);
        assert!(!expansion.contains("Result"));
        assert!(!expansion.contains("Option"));
    }

    #[test]
    fn emits_each_body_once_across_a_question_join_chain() {
        let mut function: ItemFn = parse_quote! {
            fn and(a: bool, b: bool, c: bool) -> bool {
                #[question("a")]
                #[yes("Yes")]
                #[no("No")]
                let (check_b, false_result) = |a| a;

                #[question("b")]
                #[yes("Yes")]
                #[no("No")]
                let (check_c, false_result) = |check_b, b| b;

                #[question("c")]
                #[yes("Yes")]
                #[no("No")]
                let (true_result, false_result) = |check_c, c| c;

                #[action("Return true.")]
                let end = |true_result| __kaalang_true_marker(true);

                #[action("Return false.")]
                let end = |false_result| __kaalang_false_marker(false);

                |end| return end;
            }
        };

        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert_eq!(expansion.matches("__kaalang_true_marker").count(), 1);
        assert_eq!(expansion.matches("__kaalang_false_marker").count(), 1);
    }

    #[test]
    fn a_choice_dispatches_once_without_a_case_tag() {
        let mut function: ItemFn = parse_quote! {
            fn choose(input: Option<String>) -> usize {
                #[choice("Was text supplied?")]
                #[case("Text")]
                #[case("Absent")]
                let (text, absent) = |input| {
                    match input {
                        Some(value) => value,
                        None => 0u8,
                    }
                };

                #[action("Measure the text")]
                let selected = |text| { text.len() };

                #[action("Use the fallback")]
                let selected = |absent| { absent as usize };

                #[action("Finish")]
                let end = |selected| { __kaalang_shared_marker(selected) };

                |end| return end;
            }
        };
        let expansion = expand(&mut function).expect("the flow expands").to_string();
        assert_eq!(branching_matches(&function), 1);
        assert!(!expansion.contains("Result"));
        assert_eq!(
            expansion.matches("Option").count(),
            2,
            "only the outer and internal input signatures"
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
                let (a, b, done, c, d) = |value, &condition| {
                    match value {
                        0 => (),
                        1 => (),
                        2 => (),
                        3 => (),
                        _ => (),
                    }
                };

                #[action("Build the left value from a")]
                let left = |a| { 1 };

                #[action("Build the left value from b")]
                let left = |b| { 2 };

                #[action("Produce the terminal result")]
                let end = |done| { __kaalang_terminal_marker(3) };

                #[question("Refine the right group")]
                let (yes, no) = |c, condition| { condition };

                #[action("Build the right value on yes")]
                let right = |yes| { 4 };

                #[action("Build the right value on no")]
                let right = |no| { 5 };

                #[action("Build the right value from d")]
                let right = |d| { 6 };

                #[action("Use the left value")]
                let end = |left| { __kaalang_left_marker(left) };

                #[action("Use the right value")]
                let end = |right| { __kaalang_right_marker(right) };

                |end| return end;
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
                let expression = match statement {
                    Stmt::Local(local) => local.init.as_mut().unwrap().expr.as_mut(),
                    Stmt::Expr(expression, _) => expression,
                    _ => panic!("each fixture statement declares a block"),
                };
                if matches!(expression, Expr::Break(_) | Expr::Return(_)) {
                    continue;
                }
                let Expr::Closure(closure) = expression else {
                    panic!("each block has a closure initializer");
                };
                if matches!(closure.body.as_ref(), Expr::Break(_) | Expr::Return(_)) {
                    continue;
                }
                if !matches!(closure.body.as_ref(), Expr::Block(_)) {
                    let expression = &closure.body;
                    closure.body = parse_quote!({ #expression });
                }
                let Expr::Block(body) = closure.body.as_mut() else {
                    unreachable!("the body has been wrapped in a block");
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
