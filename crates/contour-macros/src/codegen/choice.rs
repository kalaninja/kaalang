//! Emits a choice's hygienic continuations and its authored dispatch.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::{Bindings, Merged, body::block_body, input_bindings};
use contour_model::{Flow, choice_match, is_todo_body};

pub(crate) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    branches: &[TokenStream2],
    merged: Option<Merged>,
) -> TokenStream2 {
    let block = &flow.blocks[index];
    let input_bindings = input_bindings(&block.inputs, bindings);
    let body = block_body(&block.body);
    let continuations = block
        .outputs
        .iter()
        .enumerate()
        .map(|(case, _)| {
            Ident::new(
                &format!("__contour_continue_{index}_{case}"),
                Span::mixed_site(),
            )
        })
        .collect::<Vec<_>>();
    let capability_type = Ident::new(
        &format!("__ContourContinuationCapability{index}"),
        Span::mixed_site(),
    );
    let capability = Ident::new(
        &format!("__contour_continuation_capability_{index}"),
        Span::mixed_site(),
    );
    let consumed_capability = Ident::new(
        &format!("__contour_consumed_capability_{index}"),
        Span::mixed_site(),
    );
    // Definition-site hygiene keeps match bindings out of downstream blocks.
    let definitions = block
        .outputs
        .iter()
        .zip(branches)
        .zip(&continuations)
        .map(|((output, path), continuation)| {
            let output_wire = bindings.wire(output);
            quote! {
                macro_rules! #continuation {
                    ($value:expr) => {{
                        let #consumed_capability: #capability_type = #capability;
                        let #output_wire = $value;
                        #path
                    }};
                }
            }
        })
        .collect::<Vec<_>>();

    let dispatch = if is_todo_body(&block.body) {
        let numbered = continuations[..continuations.len() - 1]
            .iter()
            .enumerate()
            .map(|(case, continuation)| quote!(#case => #continuation!(todo!()),));
        let last = continuations
            .last()
            .expect("a choice has at least two cases");
        quote! {
            #[allow(clippy::diverging_sub_expression)]
            match {
                #input_bindings
                #body
            } {
                #(#numbered)*
                _ => #last!(todo!()),
            }
        }
    } else {
        let choice = choice_match(&block.body).expect("choice bodies are validated");
        let match_attrs = &choice.attrs;
        let scrutinee = &choice.expr;
        let arms = choice
            .arms
            .iter()
            .zip(&continuations)
            .map(|(arm, continuation)| {
                let attrs = &arm.attrs;
                let pattern = &arm.pat;
                let value = &arm.body;
                quote! {
                    #(#attrs)*
                    #pattern => #continuation!(#value),
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
    };

    let Some(merged) = merged else {
        return quote_spanned! {block.span=>
            struct #capability_type;
            let #capability = #capability_type;
            #(#definitions)*
            #dispatch
        };
    };
    let output_wire = bindings.wire(&flow.blocks[merged.index].outputs[0]);
    let continuation = merged.continuation;

    quote_spanned! {block.span=>
        struct #capability_type;
        let #capability = #capability_type;
        #(#definitions)*
        let #output_wire = #dispatch;
        #continuation
    }
}
