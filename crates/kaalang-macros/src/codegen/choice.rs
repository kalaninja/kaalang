//! Emits a choice's authored dispatch, the continuations of its cases, and the
//! joins its cases yield into.
//!
//! The authored `match` runs first and hands the selected case value out of
//! its arm; the case continuation runs afterwards. A case value therefore has
//! to be owned or borrow data that outlives the choice, and match bindings
//! never reach downstream blocks. The guarded schedule stores the value in the
//! output slot with the same effect.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::{Bindings, Frame, block_body, input_bindings, join};
use join::JoinRouting;
use kaalang_model::{Block, Branch, Flow, Join, choice_match, is_todo_body};

fn mint(name: &str) -> Ident {
    Ident::new(name, Span::mixed_site())
}

/// Assigns the selected case value to its output slot inside the arm.
pub(super) fn guarded(block: &Block, bindings: &Bindings) -> TokenStream2 {
    let body = authored_match(
        block,
        &super::guarded::inputs(block, bindings),
        |case, value| {
            let wire = bindings.wire(&block.outputs[case]);
            let selected = mint("__kaalang_case_value");
            let gate = bindings.gate_value(&block.outputs[case], &quote!(#selected));
            quote!({
                let #selected = #value;
                #gate
                #wire = ::core::option::Option::Some(#selected);
            })
        },
    );
    quote!(#body;)
}

/// Preserves the authored match in both structured and guarded schedules,
/// letting the caller decide what each arm evaluates to.
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
    scope: &[Frame<'_>],
) -> TokenStream2 {
    let block = &flow.blocks[index];
    let cases = block.outputs.len();
    let routing = JoinRouting::new(index, joins.len(), branches, scope);
    let inner = Frame::nest(scope, index, routing.as_ref());
    let value = mint("__kaalang_case_value");

    // Phase one: the authored match tags the selected case value and drops
    // its arm, so the value cannot borrow a match binding. Binding the value
    // first keeps a diverging placeholder out of the tag's argument position.
    let selected = authored_match(
        block,
        &input_bindings(&block.inputs, bindings),
        |case, arm_value| {
            let tagged = join::nested(quote!(#value), case, cases);
            quote!({
                let #value = #arm_value;
                #tagged
            })
        },
    );
    // Phase two: the tag selects the continuation, which binds the output wire.
    let continuations = branches.iter().enumerate().map(|(case, branch)| {
        let pattern = join::nested(quote!(#value), case, cases);
        let wire = bindings.wire(&block.outputs[case]);
        let gate = bindings.gate(&block.outputs[case]);
        let path = super::continuation(flow, branch, bindings, &inner);
        quote! {
            #pattern => {
                let #wire = #value;
                #gate
                #path
            }
        }
    });
    let dispatch = quote_spanned! {block.span=>
        match #selected {
            #(#continuations)*
        }
    };

    join::emit(
        flow,
        bindings,
        dispatch,
        joins,
        routing.as_ref(),
        block.span,
        scope,
    )
}
