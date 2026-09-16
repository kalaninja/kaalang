//! kaalang procedural macro entry point and compilation pipeline.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Error, FnArg, ItemFn, Pat, PatIdent, Result, Type, ext::IdentExt, parse_macro_input,
    spanned::Spanned,
};

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

    let prologue = prologue(function, &bindings);
    let body = codegen::flow(&model.flow, &model.execution_plan, &bindings);
    // The flow lowers in place, so it keeps the scope the author wrote it in:
    // `Self`, the surrounding generics, and the receiver all resolve as they do
    // in any other body. A nested item would see none of them.
    *function.block = syn::parse2(quote!({
        #[allow(clippy::used_underscore_binding)]
        {
            #prologue
            #body
        }
    }))?;

    Ok(quote!(#function))
}

/// Moves every named parameter into its hygienic wire binding and then puts the
/// authored name out of reach, so a block body reads only the wires it
/// captured. A wildcard parameter provides no wire, and the receiver is its own
/// wire: Rust forbids rebinding `self`, and no other spelling can reach it.
fn prologue(function: &ItemFn, bindings: &codegen::Bindings) -> TokenStream2 {
    let parameters = function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => match argument.pat.as_ref() {
                Pat::Ident(parameter) => Some((parameter, argument.ty.as_ref())),
                Pat::Wild(_) => None,
                _ => unreachable!("the parser accepts only simple bindings or wildcards"),
            },
            FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>();
    let wires = parameters
        .iter()
        .map(|(parameter, _)| wire(parameter, bindings));
    let withdrawn = parameters
        .iter()
        .map(|(parameter, ty)| withdraw(parameter, ty));

    quote!(#(#wires)* #(#withdrawn)*)
}

/// Redeclares one parameter, as it was authored and without a value. A body
/// that reaches for a wire it did not capture still type-checks, so Rust
/// reports the authored name at that body rather than somewhere downstream.
fn withdraw(parameter: &PatIdent, ty: &Type) -> TokenStream2 {
    let ident = &parameter.ident;
    // The authored `mut` comes along, so assigning to an omitted wire reports
    // only that it was never initialized.
    let mutable = &parameter.mutability;
    // `impl Trait` names no type a `let` can repeat. Withdrawing such a
    // parameter as a unit still keeps it unreachable, only less legibly.
    let ty = if names_an_opaque_type(ty.to_token_stream()) {
        quote_spanned!(ty.span()=> ())
    } else {
        ty.to_token_stream()
    };

    quote_spanned!(ident.span()=>
        #[allow(unused_variables, unused_mut)]
        let #mutable #ident: #ty;
    )
}

/// Whether a type spells `impl Trait` anywhere, including inside a tuple, an
/// array, or a generic argument.
fn names_an_opaque_type(tokens: TokenStream2) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == "impl",
        TokenTree::Group(group) => names_an_opaque_type(group.stream()),
        _ => false,
    })
}

/// One parameter's hygienic wire binding.
fn wire(parameter: &PatIdent, bindings: &codegen::Bindings) -> TokenStream2 {
    let ident = &parameter.ident;
    let name = ident.unraw();
    let wire = bindings.wire(&name);
    let mutable = parameter
        .mutability
        .as_ref()
        .and(bindings.mutability(&name));
    // An authored `mut` a later block spends on a mutable capture is never
    // exercised on the parameter itself, so touch it where the author wrote it.
    let value = if parameter.mutability.is_some() && bindings.is_mutably_captured(&name) {
        quote!({ let _ = &mut #ident; #ident })
    } else {
        quote!(#ident)
    };

    quote!(let #mutable #wire = #value;)
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
