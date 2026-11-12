//! Removes what a target does not compile from a parsed file, as rustc strips a
//! crate before it expands it.

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    Attribute, Fields, File, FnArg, GenericParam, Generics, ImplItem, Item, Meta, Signature,
    TraitItem, punctuated::Punctuated,
};

/// Answers whether a `cfg` predicate, the tokens inside `cfg(...)`, holds.
type Holds<'a> = dyn FnMut(&TokenStream) -> bool + 'a;

/// Removes from `file` what does not hold for a target: items, `impl` and trait
/// items, fields, variants, and the generic and function parameters of those
/// under a `cfg` whose predicate fails, and every item when the file's inner
/// `cfg` fails. Each `cfg_attr` whose predicate holds is replaced with the
/// attributes it carries, and one that fails is removed, so what remains reads
/// as the target compiles it. The `cfg`s that hold are removed too.
pub fn configure(file: &mut File, holds: &mut dyn FnMut(&TokenStream) -> bool) {
    if active(&mut file.attrs, holds) {
        items(&mut file.items, holds);
    } else {
        file.items.clear();
    }
}

fn items(items: &mut Vec<Item>, holds: &mut Holds<'_>) {
    items.retain_mut(|item| {
        if !attributes(item).is_none_or(|attributes| active(attributes, holds)) {
            return false;
        }
        match item {
            Item::Mod(module) => {
                if let Some((_, inner)) = &mut module.content {
                    self::items(inner, holds);
                }
            }
            Item::Fn(function) => signature(&mut function.sig, holds),
            Item::Impl(block) => {
                generics(&mut block.generics, holds);
                block.items.retain_mut(|item| match item {
                    ImplItem::Const(item) => active(&mut item.attrs, holds),
                    ImplItem::Fn(item) => {
                        signature(&mut item.sig, holds);
                        active(&mut item.attrs, holds)
                    }
                    ImplItem::Type(item) => active(&mut item.attrs, holds),
                    ImplItem::Macro(item) => active(&mut item.attrs, holds),
                    _ => true,
                });
            }
            Item::Trait(declaration) => {
                generics(&mut declaration.generics, holds);
                declaration.items.retain_mut(|item| match item {
                    TraitItem::Const(item) => active(&mut item.attrs, holds),
                    TraitItem::Fn(item) => {
                        signature(&mut item.sig, holds);
                        active(&mut item.attrs, holds)
                    }
                    TraitItem::Type(item) => active(&mut item.attrs, holds),
                    TraitItem::Macro(item) => active(&mut item.attrs, holds),
                    _ => true,
                });
            }
            Item::Type(alias) => generics(&mut alias.generics, holds),
            Item::Struct(declaration) => {
                generics(&mut declaration.generics, holds);
                fields(&mut declaration.fields, holds);
            }
            Item::Enum(declaration) => {
                generics(&mut declaration.generics, holds);
                declaration.variants = std::mem::take(&mut declaration.variants)
                    .into_iter()
                    .filter_map(|mut variant| {
                        active(&mut variant.attrs, holds).then(|| {
                            fields(&mut variant.fields, holds);
                            variant
                        })
                    })
                    .collect();
            }
            Item::Union(declaration) => {
                generics(&mut declaration.generics, holds);
                kept(&mut declaration.fields.named, holds, field_attributes);
            }
            _ => {}
        }
        true
    });
}

/// An item's attributes, unless syn could not parse the item.
fn attributes(item: &mut Item) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        Item::Const(item) => &mut item.attrs,
        Item::Enum(item) => &mut item.attrs,
        Item::ExternCrate(item) => &mut item.attrs,
        Item::Fn(item) => &mut item.attrs,
        Item::ForeignMod(item) => &mut item.attrs,
        Item::Impl(item) => &mut item.attrs,
        Item::Macro(item) => &mut item.attrs,
        Item::Mod(item) => &mut item.attrs,
        Item::Static(item) => &mut item.attrs,
        Item::Struct(item) => &mut item.attrs,
        Item::Trait(item) => &mut item.attrs,
        Item::TraitAlias(item) => &mut item.attrs,
        Item::Type(item) => &mut item.attrs,
        Item::Union(item) => &mut item.attrs,
        Item::Use(item) => &mut item.attrs,
        _ => return None,
    })
}

fn fields(fields: &mut Fields, holds: &mut Holds<'_>) {
    match fields {
        Fields::Named(named) => kept(&mut named.named, holds, field_attributes),
        Fields::Unnamed(unnamed) => kept(&mut unnamed.unnamed, holds, field_attributes),
        Fields::Unit => {}
    }
}

fn field_attributes(field: &mut syn::Field) -> &mut Vec<Attribute> {
    &mut field.attrs
}

/// Keeps a function's generic and value parameters that hold.
fn signature(signature: &mut Signature, holds: &mut Holds<'_>) {
    generics(&mut signature.generics, holds);
    kept(&mut signature.inputs, holds, |parameter| match parameter {
        FnArg::Receiver(receiver) => &mut receiver.attrs,
        FnArg::Typed(parameter) => &mut parameter.attrs,
    });
}

fn generics(generics: &mut Generics, holds: &mut Holds<'_>) {
    kept(&mut generics.params, holds, |parameter| match parameter {
        GenericParam::Lifetime(parameter) => &mut parameter.attrs,
        GenericParam::Type(parameter) => &mut parameter.attrs,
        GenericParam::Const(parameter) => &mut parameter.attrs,
    });
}

/// Keeps the elements of `list` whose attributes hold.
fn kept<T, P: Default>(
    list: &mut Punctuated<T, P>,
    holds: &mut Holds<'_>,
    attributes: impl Fn(&mut T) -> &mut Vec<Attribute>,
) {
    *list = std::mem::take(list)
        .into_iter()
        .filter_map(|mut element| active(attributes(&mut element), holds).then_some(element))
        .collect();
}

/// Expands the `cfg_attr`s among `attributes`, then removes the `cfg`s,
/// answering whether every one holds.
fn active(attributes: &mut Vec<Attribute>, holds: &mut Holds<'_>) -> bool {
    let mut expanded = Vec::with_capacity(attributes.len());
    for attribute in attributes.drain(..) {
        expand(attribute, holds, &mut expanded);
    }
    let mut active = true;
    expanded.retain(|attribute| {
        if !attribute.path().is_ident("cfg") {
            return true;
        }
        // One predicate, which rustc lets a comma follow.
        active &= attribute.meta.require_list().is_ok_and(|list| {
            matches!(split_at_commas(list.tokens.clone()).as_slice(), [predicate] if holds(predicate))
        });
        false
    });
    *attributes = expanded;
    active
}

/// Pushes `attribute` onto `expanded`, or, for a `cfg_attr`, the attributes it
/// carries when its predicate holds, themselves expanded.
fn expand(attribute: Attribute, holds: &mut Holds<'_>, expanded: &mut Vec<Attribute>) {
    let Meta::List(list) = &attribute.meta else {
        expanded.push(attribute);
        return;
    };
    if !list.path.is_ident("cfg_attr") {
        expanded.push(attribute);
        return;
    }
    let mut parts = split_at_commas(list.tokens.clone()).into_iter();
    let Some(predicate) = parts.next() else {
        return;
    };
    if !holds(&predicate) {
        return;
    }
    for part in parts {
        // rustc rejects an argument that is not an attribute.
        if let Ok(meta) = syn::parse2::<Meta>(part) {
            let carried = Attribute {
                pound_token: attribute.pound_token,
                style: attribute.style,
                bracket_token: attribute.bracket_token,
                meta,
            };
            expand(carried, holds, expanded);
        }
    }
}

/// The comma-separated parts of `tokens`, without an empty trailing one.
pub(crate) fn split_at_commas(tokens: TokenStream) -> Vec<TokenStream> {
    let mut parts = vec![TokenStream::new()];
    for token in tokens {
        match &token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => parts.push(TokenStream::new()),
            _ => token.to_tokens(parts.last_mut().expect("parts start with one")),
        }
    }
    parts.retain(|part| !part.is_empty());
    parts
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::ToTokens;

    use super::configure;

    /// `source` as a target that holds only the names in `set` compiles it.
    fn configured(source: &str, set: &[&str]) -> String {
        let mut file = syn::parse_file(source).expect("the source parses");
        configure(&mut file, &mut |predicate: &TokenStream| {
            set.contains(&predicate.to_string().as_str())
        });
        file.into_token_stream().to_string()
    }

    #[test]
    fn an_item_under_a_failing_cfg_is_removed() {
        let source = "#[cfg(unix)] struct Unix; #[cfg(windows)] struct Windows;";
        assert_eq!(configured(source, &["unix"]), "struct Unix ;");
    }

    #[test]
    fn methods_fields_and_variants_under_a_failing_cfg_are_removed() {
        let source = "
            impl Report { #[cfg(windows)] fn raw() {} #[cfg(unix)] fn fd() {} }
            trait Review { #[cfg(windows)] fn raw(); #[cfg(unix)] fn fd(); }
            struct Named { #[cfg(windows)] raw: isize, #[cfg(unix)] fd: i32 }
            struct Tuple(#[cfg(windows)] isize, #[cfg(unix)] i32);
            enum Handle { #[cfg(windows)] Raw(isize), Fd(#[cfg(windows)] isize, #[cfg(unix)] i32) }
            union Bits { #[cfg(windows)] raw: isize, #[cfg(unix)] fd: i32 }
        ";
        assert_eq!(
            configured(source, &["unix"]),
            "impl Report { fn fd () { } } trait Review { fn fd () ; } \
             struct Named { fd : i32 } struct Tuple (i32) ; \
             enum Handle { Fd (i32) } union Bits { fd : i32 }"
        );
    }

    #[test]
    fn generic_and_function_parameters_under_a_failing_cfg_are_removed() {
        let source = "
            struct Pair<#[cfg(windows)] W, #[cfg(unix)] U>(U);
            enum Either<#[cfg(windows)] W, #[cfg(unix)] U> { Only(U) }
            union Bits<#[cfg(windows)] W, #[cfg(unix)] U> { fd: U }
            type Alias<#[cfg(windows)] W, #[cfg(unix)] U> = Pair<U>;
            fn open<#[cfg(windows)] W, #[cfg(unix)] U>(#[cfg(windows)] raw: W, #[cfg(unix)] fd: U) {}
            impl<#[cfg(windows)] W, #[cfg(unix)] U> Pair<U> {
                fn close<#[cfg(windows)] 'w>(#[cfg(windows)] self, #[cfg(unix)] fd: U) {}
            }
            trait Review<#[cfg(windows)] W> { fn check(&self, #[cfg(windows)] raw: isize); }
        ";
        assert_eq!(
            configured(source, &["unix"]),
            "struct Pair < U > (U) ; enum Either < U > { Only (U) } \
             union Bits < U > { fd : U } type Alias < U > = Pair < U > ; \
             fn open < U > (fd : U) { } impl < U > Pair < U > { fn close (fd : U) { } } \
             trait Review { fn check (& self) ; }"
        );
    }

    #[test]
    fn a_cfg_may_end_with_a_comma() {
        assert_eq!(
            configured("#[cfg(unix,)] struct Unix;", &["unix"]),
            "struct Unix ;"
        );
    }

    #[test]
    fn a_holding_cfg_is_removed_with_its_item_kept() {
        assert_eq!(
            configured("#[cfg(unix)] mod sys {}", &["unix"]),
            "mod sys { }"
        );
    }

    #[test]
    fn a_holding_cfg_attr_stands_for_its_attributes() {
        let source = "#[cfg_attr(unix, path = \"unix.rs\", allow(dead_code))] mod sys;
            #[cfg_attr(windows, repr(C))] struct Plain;";
        assert_eq!(
            configured(source, &["unix"]),
            "# [path = \"unix.rs\"] # [allow (dead_code)] mod sys ; struct Plain ;"
        );
    }

    #[test]
    fn nested_cfg_attrs_expand_and_may_carry_a_cfg() {
        let source = "#[cfg_attr(unix, cfg_attr(test, repr(C)))] struct Nested;
            #[cfg_attr(unix, cfg(windows))] struct Removed;";
        assert_eq!(
            configured(source, &["unix", "test"]),
            "# [repr (C)] struct Nested ;"
        );
    }

    #[test]
    fn inline_modules_are_configured_too() {
        let source = "mod sys { #[cfg(windows)] struct Handle; struct Shared; }";
        assert_eq!(configured(source, &[]), "mod sys { struct Shared ; }");
    }

    #[test]
    fn a_failing_inner_cfg_empties_the_file() {
        assert_eq!(configured("#![cfg(windows)] struct Handle;", &[]), "");
    }

    #[test]
    fn a_failing_inner_cfg_removes_an_inline_module() {
        assert_eq!(
            configured("mod sys { #![cfg(windows)] struct Handle; }", &[]),
            ""
        );
    }
}
