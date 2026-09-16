//! The parameter bindings a lowered flow opens with: every named parameter
//! moved into its hygienic wire, and the authored name then put out of reach.

use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::{FnArg, ItemFn, Pat, PatIdent, Type, spanned::Spanned};

use super::Bindings;

/// Moves every named parameter into its hygienic wire binding and then puts the
/// authored name out of reach, so a block body reads only the wires it
/// captured. A wildcard parameter provides no wire, and the receiver is its own
/// wire: Rust forbids rebinding `self`, and no other spelling can reach it.
pub(crate) fn emit(function: &ItemFn, bindings: &Bindings) -> TokenStream2 {
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
fn wire(parameter: &PatIdent, bindings: &Bindings) -> TokenStream2 {
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
