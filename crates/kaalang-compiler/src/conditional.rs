//! Rust configures a derive's fields before calling it. A flow uses those
//! choices inside its original function, where the parameter lints also apply.

use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hasher},
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Attribute, FnArg, GenericParam, Item, ItemEnum, Meta, Pat, Result, Token,
    parse::{ParseStream, Parser},
    punctuated::Punctuated,
};

use crate::{
    codegen,
    configure::{cfg_attributes, conditional},
    datatype,
};

fn variant(i: usize) -> Ident {
    Ident::new(&format!("P{i}"), Span::mixed_site())
}

fn variants<'a>(parameters: impl Iterator<Item = &'a [Attribute]> + Clone) -> Option<TokenStream> {
    if !parameters.clone().flatten().any(conditional) {
        return None;
    }
    let mut conditional = false;
    let variants: TokenStream = parameters
        .enumerate()
        .map(|(i, attrs)| {
            let id = variant(i);
            let guards = cfg_attributes(attrs);
            conditional |= !guards.is_empty();
            quote!(#(#guards)* #id,)
        })
        .collect();
    conditional.then_some(variants)
}

pub(super) fn parameters(
    item: &Item,
    arguments: &TokenStream,
    defining_crate: impl FnOnce() -> TokenStream,
) -> Result<Option<TokenStream>> {
    let variants = if let Item::Fn(function) = item {
        variants(function.sig.inputs.iter().map(|arg| match arg {
            FnArg::Typed(arg) => arg.attrs.as_slice(),
            FnArg::Receiver(arg) => arg.attrs.as_slice(),
        }))
    } else if let Some((_, _, generics)) = datatype::parts(item) {
        variants(generics.params.iter().map(|param| match param {
            GenericParam::Type(param) => param.attrs.as_slice(),
            _ => &[],
        }))
    } else {
        None
    };
    let Some(variants) = variants else {
        return Ok(None);
    };
    let defining_crate = defining_crate();
    let source = item.to_token_stream();
    // A local module shares the caller's type namespace, including paths in authored macros.
    let mut hash = DefaultHasher::new();
    hash.write(source.to_string().as_bytes());
    let scope = Ident::new(
        &format!("__kaalang_parameters_{:x}", hash.finish()),
        Span::mixed_site(),
    );
    // Keep the authored tokens in the invocation, including any `$` in local
    // macros. Rust only needs the guards to choose the surviving parameters.
    let body = quote! {
        #[no_implicit_prelude]
        mod #scope {
            #[derive(#defining_crate::__KaalangParameters)]
            #[#defining_crate::__kaa_discard]
            enum Parameters { #variants }
        }
        #scope::apply! { (#defining_crate::__kaa_configured_parameters) #scope (#arguments) #source }
    };
    Ok(Some(if let Item::Fn(function) = item {
        let mut shell = function.clone();
        for arg in &mut shell.sig.inputs {
            if let FnArg::Typed(arg) = arg
                && let Pat::Ident(pat) = arg.pat.as_mut()
            {
                pat.ident
                    .set_span(scope.span().located_at(pat.ident.span()));
            }
        }
        shell.block = syn::parse_quote!({ #body });
        quote!(#shell)
    } else {
        let (shell, checks) = datatype::expansion(arguments, item.clone())?;
        if checks.is_empty() {
            return Ok(Some(quote!(#shell)));
        }
        let (attrs, _, _) = datatype::parts(&shell).expect("only datatypes reach this branch");
        let lints = attrs.iter().filter_map(|attr| {
            let Meta::List(list) = &attr.meta else {
                return None;
            };
            match list.path.get_ident()?.to_string().as_str() {
                "allow" | "warn" | "deny" | "forbid" => Some(quote!(#attr)),
                "expect" => {
                    // The original item owns the lint expectation.
                    let lints = &list.tokens;
                    Some(quote!(#[allow(#lints)]))
                }
                _ => None,
            }
        });
        quote!(#shell #(#lints)* const _: () = { #body };)
    }))
}

/// Forwards the original item to lowering with the parameter indices rustc kept.
#[doc(hidden)]
#[must_use]
pub fn parameter_choices(probe: &ItemEnum) -> TokenStream {
    let active = probe.variants.iter().map(|variant| &variant.ident);
    quote! {
        macro_rules! apply {
            (($($lower:tt)*) $($source:tt)*) => {
                $($lower)*! { (#(#active),*) $($source)* }
            };
        }
        pub(super) use apply;
    }
}

/// Lowers the body in the selected parameter scope, or checks a datatype's relationships.
///
/// # Errors
///
/// Returns a diagnostic for an invalid flow or datatype.
#[doc(hidden)]
pub fn configured_parameters(tokens: TokenStream) -> Result<TokenStream> {
    (|input: ParseStream<'_>| {
        let indices;
        syn::parenthesized!(indices in input);
        let active = Punctuated::<Ident, Token![,]>::parse_terminated(&indices)?
            .into_iter()
            .collect::<HashSet<_>>();
        let scope: Ident = input.parse()?;
        let attrs;
        syn::parenthesized!(attrs in input);
        let arguments: TokenStream = attrs.parse()?;
        let mut item: Item = input.parse()?;
        if let Item::Fn(mut function) = item {
            function.sig.inputs = selected(function.sig.inputs, &active);
            let body = codegen::expand_in_scope(function, scope.span())?.block;
            return Ok(quote!(#body));
        }
        let generics = match &mut item {
            Item::Struct(item) => &mut item.generics,
            Item::Enum(item) => &mut item.generics,
            Item::Union(item) => &mut item.generics,
            Item::Type(item) => &mut item.generics,
            _ => return Err(syn::Error::new_spanned(item, "expected a flow or datatype")),
        };
        generics.params = selected(std::mem::take(&mut generics.params), &active);
        let (_, checks) = datatype::expansion(&arguments, item)?;
        Ok(quote!({ #checks }))
    })
    .parse2(tokens)
}

fn selected<T>(
    parameters: Punctuated<T, Token![,]>,
    active: &HashSet<Ident>,
) -> Punctuated<T, Token![,]> {
    parameters
        .into_iter()
        .enumerate()
        .filter_map(|(i, param)| active.contains(&variant(i)).then_some(param))
        .collect()
}
