//! Emits a choice's authored dispatch, the continuations of its cases, and the
//! joins its cases yield into.
//!
//! The authored `match` runs first and hands the selected case value out of
//! its arm; the case continuation runs afterwards. A case value therefore has
//! to be owned or borrow data that outlives the choice, and match bindings
//! never reach downstream blocks.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Expr, Lifetime};

use super::{Bindings, input_bindings, join};
use crate::{Block, Branch, Flow, Join, choice_match};

/// Rebuilds the authored match, letting the caller decide what each arm
/// evaluates to.
fn authored_match(
    block: &Block,
    input_bindings: &TokenStream2,
    arm_value: impl Fn(usize, &Expr) -> TokenStream2,
) -> TokenStream2 {
    let choice = choice_match(&block.body).expect("choice bodies are validated");
    let match_attrs = &choice.attrs;
    let scrutinee = &choice.expr;
    let arms = choice.arms.iter().enumerate().map(|(case, arm)| {
        let attrs = &arm.attrs;
        let pattern = &arm.pat;
        let value = arm_value(case, &arm.body);
        quote! {
            #(#attrs)*
            #pattern => #value,
        }
    });

    quote! {
        {
            #input_bindings
            #(#match_attrs)*
            match #scrutinee {
                #(#arms)*
            }
        }
    }
}

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: &[Branch],
    joins: &[Join],
) -> TokenStream2 {
    let block = &flow.blocks[index];
    let cases = block.outputs.len();
    // The hygienic binding that carries the selected case value out of its arm.
    let value = Ident::new("__kaalang_case_value", Span::mixed_site());
    let unreached = Ident::new("__kaalang_unreached", Span::mixed_site());
    let labels = (0..cases)
        .map(|case| {
            Lifetime::new(
                &format!("'__kaalang_case_{index}_{case}"),
                Span::mixed_site(),
            )
        })
        .collect::<Vec<_>>();

    // Each arm exits to its own binding, ending the match and input scopes
    // before its continuation. Separate labels allow different output types.
    let mut dispatch = authored_match(
        block,
        &input_bindings(&block.inputs, bindings),
        |case, arm| {
            let label = &labels[case];
            // An arm such as `_ => unreachable!()` diverges, and Rust would
            // report the `break` after it. Beside a branch that never runs, the
            // value keeps a type of its own, so nothing diverges. A `()` arm
            // stays as written: in braces clippy would report it as unneeded.
            let arm = if matches!(arm, Expr::Tuple(unit) if unit.elems.is_empty()) {
                quote!(#arm)
            } else {
                quote!(if true { #arm } else {
                    const fn #unreached<T>() -> T { ::core::unreachable!() }
                    #unreached()
                })
            };
            quote!({
                let #value = #arm;
                break #label #value;
            })
        },
    );
    dispatch = super::exits(flow, bindings, index, &labels, branches, dispatch);
    join::emit(flow, bindings, index, joins, dispatch)
}
