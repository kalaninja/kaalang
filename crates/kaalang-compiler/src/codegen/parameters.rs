//! The parameter bindings a lowered flow opens with: every named parameter
//! moved into its hygienic wire, with authored parameter bindings hidden.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{FnArg, ItemFn, Pat, PatIdent, ext::IdentExt};

use super::Bindings;

/// Moves every named parameter into its hygienic wire binding. A wildcard
/// parameter provides no wire. The receiver is its own wire: Rust forbids
/// rebinding `self`, and no other spelling can reach it.
pub(crate) fn emit(function: &mut ItemFn, bindings: &Bindings) -> TokenStream2 {
    let wires = function
        .sig
        .inputs
        .iter_mut()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => match argument.pat.as_mut() {
                Pat::Ident(parameter) => {
                    // Preserve signature spelling and source lints while keeping
                    // the binding out of authored computational bodies.
                    parameter
                        .ident
                        .set_span(Span::mixed_site().located_at(parameter.ident.span()));
                    Some(wire(parameter, bindings))
                }
                Pat::Wild(_) => None,
                _ => unreachable!("the parser accepts only simple bindings or wildcards"),
            },
            FnArg::Receiver(_) => None,
        });
    quote!(#(#wires)*)
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
    let value = if parameter.mutability.is_some() && bindings.mutability(&name).is_some() {
        quote!({ let _ = &mut #ident; #ident })
    } else {
        quote!(#ident)
    };

    quote!(let #mutable #wire = #value;)
}
