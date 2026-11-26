//! Identifies kaalang attributes in source and lets rustc resolve other macros during expansion.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{AttrStyle, Attribute, Error, Item, Meta, Result};

use crate::{conditional, configure, datatype, datatype::expand_type, expand, parse};

/// Whether `attribute` is kaalang's own: its path ends in `kaa`, as `#[kaa]`
/// and `#[kaalang::kaa]` do.
#[must_use]
pub(crate) fn is_kaalang_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "kaa" || segment.ident == "r#kaa")
}

/// Expands the attribute, letting rustc evaluate conditional parameters before
/// reading a flow or a datatype that has them.
///
/// # Errors
///
/// Returns a source-spanned error for an invalid declaration.
#[doc(hidden)]
pub fn expand_attribute(
    arguments: &TokenStream,
    mut item: Item,
    defining_crate: impl Fn() -> TokenStream,
) -> Result<TokenStream> {
    let name = match &item {
        Item::Fn(function) => Some(&function.sig.ident),
        _ => datatype::parts(&item).map(|(_, name, _)| name),
    };
    if let Some(name) = name {
        let call = name.span().located_at(proc_macro2::Span::call_site());
        let function = matches!(item, Item::Fn(_));
        let attrs =
            configure::attributes(&mut item).expect("a function or datatype has attributes");
        if already_applied(attrs) {
            let message = if function {
                DUPLICATE_FLOW
            } else {
                DUPLICATE_TYPE
            };
            let error = Error::new(call, message);
            if function {
                return Err(error);
            }
            let error = error.into_compile_error();
            return Ok(quote!(#item #error));
        }
        if function && !arguments.is_empty() {
            return Err(Error::new(proc_macro2::Span::call_site(), FLOW_ARGUMENTS));
        }
        if attrs.iter().any(|attr| {
            matches!(attr.style, AttrStyle::Outer)
                && !BUILT_IN.iter().any(|name| attr.path().is_ident(name))
                && parse::annotation_name(attr).is_none()
        }) {
            // rustc resolves the remaining attribute macros, and any of them
            // may be our macro under another name.
            let owner = defining_crate();
            attrs.push(syn::parse_quote!(#[#owner::__kaa_applied]));
        }
    }
    if let Some(expansion) = conditional::parameters(&item, arguments, defining_crate)? {
        return Ok(expansion);
    }
    match item {
        Item::Fn(function) => expand(function).map(ToTokens::into_token_stream),
        item => expand_type(arguments, item),
    }
}

/// Built-in attributes, which rustc never takes for an imported macro, so none
/// of them is a second `kaa`.
const BUILT_IN: [&str; 13] = [
    "allow",
    "cfg",
    "cold",
    "deny",
    "deprecated",
    "doc",
    "expect",
    "forbid",
    "inline",
    "must_use",
    "non_exhaustive",
    "repr",
    "warn",
];

const FLOW_ARGUMENTS: &str = "#[kaa] on a function does not accept arguments";
const DUPLICATE_FLOW: &str = "a function takes one `#[kaa]` attribute";
pub(crate) const DUPLICATE_TYPE: &str = "a type takes one `#[kaa]` attribute";

pub(crate) fn already_applied(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attr| {
        let path = attr.path();
        path.segments.len() == 2
            && path.segments[0].ident == "$crate"
            && path.segments[1].ident == "__kaa_applied"
    })
}

pub(crate) fn flow_attributes(attributes: &[Attribute]) -> Result<()> {
    if already_applied(attributes) {
        return Ok(());
    }
    let mut seen = false;
    for attr in attributes.iter().filter(|attr| is_kaalang_attribute(attr)) {
        if seen {
            return Err(Error::new_spanned(attr, DUPLICATE_FLOW));
        }
        seen = true;
        match &attr.meta {
            Meta::Path(_) => {}
            Meta::List(list) if list.tokens.is_empty() => {}
            _ => return Err(Error::new_spanned(attr, FLOW_ARGUMENTS)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::expand_attribute;

    /// Whether expanding `item` marks it for a second `kaa` to find.
    fn marked(item: &str) -> bool {
        let item = syn::parse_str(item).expect("the item parses");
        expand_attribute(&quote!(), item, || quote!(::kaalang))
            .expect("the item expands")
            .to_string()
            .contains("__kaa_applied")
    }

    #[test]
    fn an_attribute_that_may_be_a_renamed_kaa_marks_the_item() {
        assert!(marked("#[derive(Debug)] struct Member { id: u8 }"));
    }

    #[test]
    fn built_in_attributes_leave_the_item_unmarked() {
        assert!(!marked(
            "/// A member.\n#[allow(dead_code)]\n#[repr(C)]\n#[must_use]\nstruct Member { id: u8 }"
        ));
    }
}
