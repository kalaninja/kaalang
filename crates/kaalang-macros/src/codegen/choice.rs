//! Emits a choice's authored dispatch, the continuations of its cases, and the
//! joins its cases yield into.
//!
//! The authored `match` runs first and hands the selected case value out of
//! its arm; the case continuation runs afterwards. A case value therefore has
//! to be owned or borrow data that outlives the choice, and match bindings
//! never reach downstream blocks.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::Lifetime;

use super::{Bindings, block_body, input_bindings, join};
use kaalang_model::{Block, Branch, Flow, Join, choice_match, is_todo_body};

/// The hygienic binding that carries the selected case value out of its arm.
fn case_value() -> Ident {
    Ident::new("__kaalang_case_value", Span::mixed_site())
}

/// Rebuilds the authored match, letting the caller decide what each arm
/// evaluates to.
fn authored_match(
    block: &Block,
    input_bindings: &TokenStream2,
    arm_value: impl Fn(usize, TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    if is_todo_body(&block.body) {
        let body = block_body(&block.body);
        let numbered = (0..block.outputs.len() - 1).map(|case| {
            let value = arm_value(case, quote!(todo!()));
            quote!(#case => #value,)
        });
        let last = arm_value(block.outputs.len() - 1, quote!(todo!()));
        // The braces make the attribute sit on a statement: a join binds the
        // dispatch as an initializer, where an attribute on a bare expression
        // is rejected.
        return quote! {{
            #[allow(clippy::diverging_sub_expression)]
            match {
                #input_bindings
                #body
            } {
                #(#numbered)*
                _ => #last,
            }
        }};
    }

    let choice = choice_match(&block.body).expect("choice bodies are validated");
    let match_attrs = &choice.attrs;
    let scrutinee = &choice.expr;
    let arms = choice.arms.iter().enumerate().map(|(case, arm)| {
        let attrs = &arm.attrs;
        let pattern = &arm.pat;
        let value = &arm.body;
        let value = arm_value(case, quote!(#value));
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
    let value = case_value();
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
        |case, arm_value| {
            let label = &labels[case];
            quote!({
                let #value = #arm_value;
                break #label #value;
            })
        },
    );
    // Each continuation leaves for a join or returns the flow result, so it
    // cannot fall through into the continuation of a different case.
    for (case, branch) in branches.iter().enumerate() {
        let label = &labels[case];
        let wire = bindings.pattern(block.output_span, &block.outputs[case..=case]);
        let gate = bindings.gate(&block.outputs[case]);
        let path = super::flow(flow, &branch.plan, bindings);
        dispatch = quote! {
            let #wire = #label: { #dispatch };
            #gate
            #path
        };
    }
    join::emit(flow, bindings, index, joins, dispatch)
}
