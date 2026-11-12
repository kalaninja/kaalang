//! Parses type declarations with their annotations, and expands datatypes for
//! the macro.

use std::iter;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Error, Generics, Ident, Item, LitStr, Meta, Path, Result, Token, Type, Visibility,
    parse::ParseStream, spanned::Spanned,
};

use crate::{
    configure::split_at_commas,
    is_kaalang_attribute,
    parse::{annotation_name, description, path_annotation_name},
};

/// A struct, enum, union, or type alias, with its annotations if it has any.
pub struct TypeDeclaration {
    pub generics: Generics,
    /// The description from `#[kaa("description")]`, if any.
    pub description: Option<String>,
    pub form: TypeForm,
}

/// The declaration form with its immediate structure.
pub enum TypeForm {
    Struct(Vec<Field>),
    Enum(Vec<Variant>),
    Union(Vec<Field>),
    /// The right-hand side and the alias's own relationship, if any.
    Alias(Box<Type>, Option<Relationship>),
}

pub struct Variant {
    pub name: Ident,
    /// The description from `#[kaa::case("description")]`, if any.
    pub case: Option<String>,
    pub fields: Vec<Field>,
}

pub struct Field {
    /// `None` for a tuple field, which is named by its position.
    pub name: Option<Ident>,
    /// As declared; `Inherited` on every enum-variant field.
    pub visibility: Visibility,
    pub ty: Type,
    pub annotation: Option<Annotation>,
}

pub enum Annotation {
    Value(String),
    Refer(Relationship),
}

pub struct Relationship {
    /// The target as written, a name or a path without parameters, with `Self`
    /// replaced by the declaring type's name.
    pub target: Path,
    pub description: String,
    /// The `cfg` attributes of the field and its variant: the target exists
    /// only where they hold.
    pub cfgs: Vec<Attribute>,
}

impl TypeDeclaration {
    /// Every relationship in source order, the position that identifies it.
    #[must_use]
    pub fn relationships(&self) -> Vec<&Relationship> {
        let fields: Vec<&Field> = match &self.form {
            TypeForm::Struct(fields) | TypeForm::Union(fields) => fields.iter().collect(),
            TypeForm::Enum(variants) => variants
                .iter()
                .flat_map(|variant| &variant.fields)
                .collect(),
            TypeForm::Alias(_, relationship) => return relationship.iter().collect(),
        };
        fields
            .into_iter()
            .filter_map(|field| match &field.annotation {
                Some(Annotation::Refer(relationship)) => Some(relationship),
                _ => None,
            })
            .collect()
    }
}

const UNSUPPORTED: &str =
    "#[kaa] applies to functions with a body, structs, enums, unions, and type aliases";

/// Parses a type declaration with its annotations, if it has any. Its first
/// `#[kaa]` attribute supplies the optional description.
///
/// # Errors
///
/// Returns the first annotation error at its source span.
pub fn type_declaration(item: &Item) -> Result<TypeDeclaration> {
    let Some((attributes, name, generics)) = parts(item) else {
        return Err(Error::new_spanned(item, UNSUPPORTED));
    };
    let mut kaalang = attributes
        .iter()
        .filter(|attribute| is_kaalang_attribute(attribute));
    // Read as the macro reads it: only the tokens inside the delimiter, whichever
    // delimiter it is, and none at all as no description.
    let description = match kaalang.next().map(|attribute| (attribute, &attribute.meta)) {
        None | Some((_, Meta::Path(_))) => None,
        Some((_, Meta::List(list))) if list.tokens.is_empty() => None,
        Some((_, Meta::List(list))) => {
            let tokens = &list.tokens;
            let normalized: Attribute = syn::parse_quote_spanned!(list.span()=> #[kaa(#tokens)]);
            Some(description(&normalized, "kaalang type")?)
        }
        Some((attribute, Meta::NameValue(_))) => Some(description(attribute, "kaalang type")?),
    };
    if let Some(duplicate) = kaalang.next() {
        return Err(Error::new_spanned(
            duplicate,
            "a type takes one `#[kaa]` attribute",
        ));
    }

    let mut alias_relationship = None;
    for attribute in attributes {
        match annotation(attribute)? {
            Some("value") => {
                return Err(Error::new_spanned(
                    attribute,
                    "`value` describes a field; a type takes its description from `#[kaa(\"description\")]`",
                ));
            }
            Some("case") => return Err(misplaced_case(attribute)),
            Some("refer") if !matches!(item, Item::Type(_)) => {
                return Err(Error::new_spanned(
                    attribute,
                    "only a type alias takes a declaration-level `refer`; put it on a field",
                ));
            }
            Some("refer") if alias_relationship.is_some() => {
                return Err(Error::new_spanned(
                    attribute,
                    "a type alias takes one `refer` annotation",
                ));
            }
            Some("refer") => {
                alias_relationship = Some(relationship(attribute, None, generics, Vec::new())?);
            }
            _ => {}
        }
    }

    let form = match item {
        Item::Struct(declaration) => {
            TypeForm::Struct(fields(&declaration.fields, name, generics, &[])?)
        }
        Item::Enum(declaration) => TypeForm::Enum(
            declaration
                .variants
                .iter()
                .map(|variant| {
                    let cfgs = cfg_attributes(&variant.attrs);
                    Ok(Variant {
                        name: variant.ident.clone(),
                        case: variant_case(&variant.attrs)?,
                        fields: fields(&variant.fields, name, generics, &cfgs)?,
                    })
                })
                .collect::<Result<_>>()?,
        ),
        Item::Union(declaration) => {
            TypeForm::Union(fields(&declaration.fields.named, name, generics, &[])?)
        }
        Item::Type(declaration) => TypeForm::Alias(declaration.ty.clone(), alias_relationship),
        _ => unreachable!("unsupported forms were rejected above"),
    };
    Ok(TypeDeclaration {
        generics: generics.clone(),
        description,
        form,
    })
}

/// Collects the type declarations from these items, datatypes and plain types
/// alike, in source order, each with its path among them: `Member`, or
/// `directory::Team` inside an inline module.
#[must_use]
pub fn type_items(items: &[Item]) -> Vec<(String, Item)> {
    let mut types = Vec::new();
    collect_types(items, "", &mut types);
    types
}

fn collect_types(items: &[Item], prefix: &str, types: &mut Vec<(String, Item)>) {
    for item in items {
        if let Item::Mod(module) = item
            && let Some((_, items)) = &module.content
        {
            collect_types(items, &format!("{prefix}{}::", module.ident), types);
        } else if let Some((_, name, _)) = parts(item) {
            types.push((format!("{prefix}{name}"), item.clone()));
        }
    }
}

/// The attributes, name, and generics of a supported declaration form.
fn parts(item: &Item) -> Option<(&Vec<Attribute>, &Ident, &Generics)> {
    match item {
        Item::Struct(declaration) => Some((
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        )),
        Item::Enum(declaration) => Some((
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        )),
        Item::Union(declaration) => Some((
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        )),
        Item::Type(declaration) => Some((
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        )),
        _ => None,
    }
}

/// Expands a datatype for the macro: the declaration without its
/// annotations, then an import of each relationship target, so that rustc
/// resolves the target as the declaration's module sees it.
///
/// # Errors
///
/// Rejects unsupported declaration forms. Annotation errors are emitted at
/// their source spans under the same compilation guards as the annotations.
pub fn expand_type(arguments: &TokenStream, mut item: Item) -> Result<TokenStream> {
    // Spanned at the arguments, so that a malformed description points at them.
    let attribute: Attribute = match arguments.clone().into_iter().next() {
        None => syn::parse_quote!(#[kaa]),
        Some(first) => syn::parse_quote_spanned!(first.span()=> #[kaa(#arguments)]),
    };
    let mut checks = TokenStream::new();
    let attributes = match &mut item {
        Item::Struct(declaration) => {
            checks.extend(expand_fields(
                &mut declaration.fields,
                &declaration.ident,
                &declaration.generics,
                &[],
            ));
            &mut declaration.attrs
        }
        Item::Enum(declaration) => {
            for variant in &mut declaration.variants {
                let cfgs = cfg_attributes(&variant.attrs);
                checks.extend(expand_annotations(&mut variant.attrs, &[], |attributes| {
                    variant_case(attributes).map(|_| None)
                }));
                checks.extend(expand_fields(
                    &mut variant.fields,
                    &declaration.ident,
                    &declaration.generics,
                    &cfgs,
                ));
            }
            &mut declaration.attrs
        }
        Item::Union(declaration) => {
            checks.extend(expand_fields(
                &mut declaration.fields.named,
                &declaration.ident,
                &declaration.generics,
                &[],
            ));
            &mut declaration.attrs
        }
        Item::Type(declaration) => &mut declaration.attrs,
        // Rejected before validation, so that the span leaves out the inserted attribute.
        _ => return Err(Error::new_spanned(item, UNSUPPORTED)),
    };
    let root: Vec<_> = attributes
        .iter()
        .filter(|attribute| annotation_name(attribute).is_some())
        .cloned()
        .collect();
    attributes.retain(|attribute| annotation_name(attribute).is_none());
    let annotated: Item = syn::parse_quote!(#attribute #(#root)* #item);
    let declaration = match type_declaration(&annotated) {
        Ok(declaration) => declaration,
        // Without its annotations the item is valid Rust, so its uses still resolve.
        Err(error) => {
            let error = error.into_compile_error();
            return Ok(quote! { #item #error });
        }
    };

    // Each import is spanned at its target, where rustc reports an unknown one,
    // and stands under its field's `cfg`, as the target may.
    checks.extend(
        declaration
            .relationships()
            .into_iter()
            .map(|relationship| {
                let (target, cfgs) = (&relationship.target, &relationship.cfgs);
                quote_spanned!(target.span()=> #(#cfgs)* #[allow(unused_imports)] use #target as _;)
            })
            .collect::<TokenStream>(),
    );
    let checks = (!checks.is_empty()).then(|| quote! { const _: () = { #checks }; });
    Ok(quote! { #item #checks })
}

fn expand_fields<'a>(
    fields: impl IntoIterator<Item = &'a mut syn::Field>,
    owner: &Ident,
    generics: &Generics,
    outer: &[Attribute],
) -> TokenStream {
    let mut checks = TokenStream::new();
    for field in fields {
        let mut read = field.clone();
        checks.extend(expand_annotations(&mut field.attrs, outer, |attributes| {
            read.attrs = attributes.to_vec();
            let parsed = self::fields(iter::once(&read), owner, generics, &[])?;
            Ok(parsed
                .into_iter()
                .next()
                .and_then(|field| match field.annotation {
                    Some(Annotation::Refer(relationship)) => Some(relationship.target),
                    _ => None,
                }))
        }));
    }
    checks
}

/// Rust evaluates the guards on each check, including whether two annotations
/// coexist; the procedural macro itself runs for the host, not the target.
fn expand_annotations(
    attributes: &mut Vec<Attribute>,
    outer: &[Attribute],
    mut validate: impl FnMut(&[Attribute]) -> Result<Option<Path>>,
) -> TokenStream {
    let (kept, annotations) = lift_annotations(attributes);
    *attributes = kept;
    let cfgs: Vec<_> = outer
        .iter()
        .cloned()
        .chain(cfg_attributes(attributes))
        .collect();
    let mut checks = TokenStream::new();
    let mut valid = Vec::new();
    for (predicates, annotation) in &annotations {
        let guarded = |check: TokenStream, other: &[TokenStream]| {
            let condition = (!predicates.is_empty() || !other.is_empty()).then(|| {
                let predicates = predicates.iter().chain(other);
                quote!(#[cfg(all(#(#predicates),*))])
            });
            quote! { #(#cfgs)* #condition #check }
        };
        match validate(std::slice::from_ref(annotation)) {
            Ok(target) => {
                if let Some(target) = target {
                    checks.extend(guarded(
                        quote_spanned!(target.span()=> #[allow(unused_imports)] use #target as _;),
                        &[],
                    ));
                }
                for &(previous, prior) in &valid {
                    if let Err(error) = validate(&[Attribute::clone(prior), annotation.clone()]) {
                        checks.extend(guarded(error.into_compile_error(), previous));
                    }
                }
                valid.push((predicates.as_slice(), annotation));
            }
            Err(error) => checks.extend(guarded(error.into_compile_error(), &[])),
        }
    }
    checks
}

/// Removes annotations, retaining the predicates around each separately.
fn lift_annotations(
    attributes: &[Attribute],
) -> (Vec<Attribute>, Vec<(Vec<TokenStream>, Attribute)>) {
    let mut kept = Vec::new();
    let mut read = Vec::new();
    for attribute in attributes {
        if annotation_name(attribute).is_some() {
            read.push((Vec::new(), attribute.clone()));
            continue;
        }
        let Meta::List(list) = &attribute.meta else {
            kept.push(attribute.clone());
            continue;
        };
        if !list.path.is_ident("cfg_attr") {
            kept.push(attribute.clone());
            continue;
        }
        let mut lifted = Vec::new();
        let rest = take_annotations(list.tokens.clone(), &mut Vec::new(), &mut lifted);
        if let Some(rest) = rest {
            let rest = Attribute {
                meta: syn::parse_quote_spanned!(list.path.span()=> cfg_attr(#rest)),
                ..attribute.clone()
            };
            kept.push(rest);
        }
        for (predicates, annotation) in lifted {
            read.push((
                predicates,
                Attribute {
                    meta: annotation,
                    ..attribute.clone()
                },
            ));
        }
    }
    (kept, read)
}

/// Takes the annotations out of the arguments of a `cfg_attr`, nested ones
/// included, pushing each onto `lifted` with the predicates around it, and
/// returns the arguments left, unless no attribute is.
fn take_annotations(
    arguments: TokenStream,
    predicates: &mut Vec<TokenStream>,
    lifted: &mut Vec<(Vec<TokenStream>, Meta)>,
) -> Option<TokenStream> {
    let mut parts = split_at_commas(arguments).into_iter();
    let predicate = parts.next()?;
    predicates.push(predicate.clone());
    let mut rest = Vec::new();
    for part in parts {
        match syn::parse2::<Meta>(part.clone()) {
            Ok(Meta::List(nested)) if nested.path.is_ident("cfg_attr") => {
                if let Some(nested) = take_annotations(nested.tokens, predicates, lifted) {
                    rest.push(quote!(cfg_attr(#nested)));
                }
            }
            Ok(meta) if path_annotation_name(meta.path()).is_some() => {
                lifted.push((predicates.clone(), meta));
            }
            _ => rest.push(part),
        }
    }
    predicates.pop();
    (!rest.is_empty()).then(|| quote!(#predicate, #(#rest),*))
}

/// The type annotation `attribute` names: `kaa::value`, `kaa::refer`, or
/// `kaa::case`. Any other name under `kaa::` is rejected.
fn annotation(attribute: &Attribute) -> Result<Option<&'static str>> {
    let Some(name) = annotation_name(attribute) else {
        return Ok(None);
    };
    let known = ["value", "refer", "case"]
        .into_iter()
        .find(|known| name == known);
    known.map(Some).ok_or_else(|| {
        Error::new_spanned(
            attribute,
            format!("`kaa::{name}` is not a kaalang type annotation"),
        )
    })
}

/// Parses the fields of a struct, union, or variant of the type `owner`, whose
/// `cfg` attributes are `outer`.
fn fields<'a>(
    fields: impl IntoIterator<Item = &'a syn::Field>,
    owner: &Ident,
    generics: &Generics,
    outer: &[Attribute],
) -> Result<Vec<Field>> {
    fields
        .into_iter()
        .map(|field| {
            let mut annotation = None;
            for attribute in &field.attrs {
                let parsed = match self::annotation(attribute)? {
                    Some("value") => Annotation::Value(description(attribute, "kaalang value")?),
                    Some("refer") => {
                        let cfgs = outer
                            .iter()
                            .cloned()
                            .chain(cfg_attributes(&field.attrs))
                            .collect();
                        Annotation::Refer(relationship(attribute, Some(owner), generics, cfgs)?)
                    }
                    Some(_) => return Err(misplaced_case(attribute)),
                    None => continue,
                };
                if annotation.replace(parsed).is_some() {
                    return Err(Error::new_spanned(
                        attribute,
                        "a field takes one `value` or `refer` annotation",
                    ));
                }
            }
            Ok(Field {
                name: field.ident.clone(),
                visibility: field.vis.clone(),
                ty: field.ty.clone(),
                annotation,
            })
        })
        .collect()
}

fn variant_case(attributes: &[Attribute]) -> Result<Option<String>> {
    let mut case = None;
    for attribute in attributes {
        match annotation(attribute)? {
            Some("case") => {
                if case
                    .replace(description(attribute, "kaalang case")?)
                    .is_some()
                {
                    return Err(Error::new_spanned(
                        attribute,
                        "an enum variant takes one `case` annotation",
                    ));
                }
            }
            Some(_) => {
                return Err(Error::new_spanned(
                    attribute,
                    "`value` and `refer` describe fields; an enum variant takes `#[kaa::case(\"description\")]`",
                ));
            }
            None => {}
        }
    }
    Ok(case)
}

fn misplaced_case(attribute: &Attribute) -> Error {
    Error::new_spanned(attribute, "`case` describes an enum variant")
}

fn cfg_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    fn guard(meta: &Meta) -> Option<Meta> {
        let Meta::List(list) = meta else {
            return None;
        };
        if list.path.is_ident("cfg") {
            return Some(meta.clone());
        }
        if !list.path.is_ident("cfg_attr") {
            return None;
        }
        let mut parts = split_at_commas(list.tokens.clone()).into_iter();
        let predicate = parts.next()?;
        let guards: Vec<_> = parts
            .filter_map(|part| syn::parse2::<Meta>(part).ok().and_then(|meta| guard(&meta)))
            .collect();
        (!guards.is_empty())
            .then(|| syn::parse_quote_spanned!(list.span()=> cfg_attr(#predicate, #(#guards),*)))
    }
    attributes
        .iter()
        .filter_map(|attribute| {
            guard(&attribute.meta).map(|meta| Attribute {
                meta,
                ..attribute.clone()
            })
        })
        .collect()
}

/// Parses `refer(Target, "description")` on a field of the type `owner` under
/// `cfgs`, or on a type alias, which has no owner.
fn relationship(
    attribute: &Attribute,
    owner: Option<&Ident>,
    generics: &Generics,
    cfgs: Vec<Attribute>,
) -> Result<Relationship> {
    let malformed = || {
        Error::new_spanned(
            attribute,
            "a relationship is written `#[kaa::refer(Target, \"description\")]`",
        )
    };
    let Meta::List(list) = &attribute.meta else {
        return Err(malformed());
    };
    let (mut target, description) = list
        .parse_args_with(|input: ParseStream| {
            let target: Path = input.parse()?;
            input.parse::<Token![,]>()?;
            Ok((target, input.parse::<LitStr>()?))
        })
        .map_err(|_| malformed())?;
    if description.value().trim().is_empty() {
        return Err(Error::new_spanned(
            description,
            "a kaalang relationship requires a non-empty description",
        ));
    }
    if let Some(segment) = target
        .segments
        .iter()
        .find(|segment| !segment.arguments.is_none())
    {
        return Err(Error::new_spanned(
            &segment.arguments,
            "a relationship target is written without parameters",
        ));
    }
    if target.is_ident("Self") {
        // As in Rust, `Self` names the type in its fields, and an alias has none.
        let Some(owner) = owner else {
            return Err(Error::new_spanned(&target, "a type alias has no `Self`"));
        };
        // Spanned at `Self`, so that the card shows the target as written.
        let mut name = owner.clone();
        name.set_span(target.span());
        target = name.into();
    }
    if let Some(name) = target.get_ident()
        && generics
            .type_params()
            .any(|parameter| parameter.ident == *name)
    {
        return Err(Error::new_spanned(
            name,
            "a type parameter is not a relationship target",
        ));
    }
    Ok(Relationship {
        target,
        description: description.value(),
        cfgs,
    })
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::ToTokens;
    use syn::{Item, parse_quote};

    use super::{TypeForm, expand_type, type_declaration, type_items};

    fn targets(item: &Item) -> Vec<String> {
        type_declaration(item)
            .unwrap()
            .relationships()
            .into_iter()
            .map(|relationship| relationship.target.to_token_stream().to_string())
            .collect()
    }

    #[test]
    fn relationships_follow_source_order_across_variants() {
        let item = parse_quote! {
            #[kaa]
            enum Document {
                Article(#[kaa::refer(Article, "The content.")] Body, #[kaa::value("A slug.")] String),
                #[kaa::case("Nothing yet.")]
                Empty,
                Report { #[kaa::refer(crate::Report, "The report.")] report: Body },
            }
        };
        assert_eq!(targets(&item), ["Article", "crate :: Report"]);
    }

    #[test]
    fn an_alias_owns_its_declaration_level_relationship() {
        let item = parse_quote! {
            #[kaa("Earlier descriptions.")]
            #[kaa::refer(RevisionDetails, "In revision order.")]
            type History = Vec<RevisionDetails>;
        };
        assert_eq!(targets(&item), ["RevisionDetails"]);
        let declaration = type_declaration(&item).unwrap();
        assert_eq!(
            declaration.description.as_deref(),
            Some("Earlier descriptions.")
        );
        assert!(matches!(declaration.form, TypeForm::Alias(..)));
    }

    #[test]
    fn expansion_strips_annotations_and_imports_each_target() {
        let item = parse_quote! {
            #[derive(Debug)]
            struct Article<'a> {
                #[kaa::value("The title.")]
                title: String,
                #[kaa::refer(super::Member, "The owner.")]
                #[doc = "kept"]
                owner: &'a Member,
            }
        };
        let expanded = expand_type(&quote::quote!("An article."), item)
            .unwrap()
            .to_string();
        for removed in ["kaa", "value", "refer"] {
            assert!(!expanded.contains(removed), "{expanded}");
        }
        assert!(expanded.contains("derive (Debug)"), "{expanded}");
        assert!(expanded.contains("doc = \"kept\""), "{expanded}");
        assert!(
            expanded.contains("use super :: Member as _ ;"),
            "{expanded}"
        );
    }

    #[test]
    fn an_annotation_under_a_cfg_attr_checks_its_target_under_the_predicate() {
        let item = parse_quote! {
            struct Session {
                #[cfg_attr(unix, kaa::refer(Device, "The device."), allow(dead_code))]
                device: u8,
                #[cfg_attr(unix, cfg_attr(test, kaa::value("A log.")))]
                log: String,
            }
        };
        let expanded = expand_type(&TokenStream::new(), item).unwrap().to_string();
        assert!(!expanded.contains("kaa"), "{expanded}");
        assert!(
            expanded.contains("# [cfg_attr (unix , allow (dead_code))] device : u8 , log : String"),
            "{expanded}"
        );
        assert!(
            expanded.contains("# [cfg (all (unix))] # [allow (unused_imports)] use Device as _ ;"),
            "{expanded}"
        );
    }

    #[test]
    fn a_rejected_annotation_keeps_the_type_beside_its_error() {
        let item = parse_quote! {
            struct Article {
                #[kaa::vlaue("The title.")]
                title: String,
            }
        };
        let expanded = expand_type(&TokenStream::new(), item).unwrap().to_string();
        assert!(expanded.starts_with("struct Article"), "{expanded}");
        assert!(expanded.contains("compile_error"), "{expanded}");
        assert!(!expanded.contains("vlaue ("), "{expanded}");
    }

    /// An attribute macro sees a field's `cfg` as written, so its import must
    /// hold under the same one: the target may be gone with the field.
    #[test]
    fn expansion_keeps_a_field_cfg_and_its_variant_cfg_on_the_import() {
        let item = parse_quote! {
            enum Record {
                #[cfg(feature = "audit")]
                Audited {
                    #[cfg(not(test))]
                    #[kaa::refer(Auditor, "Who audited.")]
                    by: u8,
                },
            }
        };
        let expanded = expand_type(&TokenStream::new(), item).unwrap().to_string();
        assert!(
            expanded.contains(
                "# [cfg (feature = \"audit\")] # [cfg (not (test))] # [allow (unused_imports)] use Auditor as _ ;"
            ),
            "{expanded}"
        );
    }

    #[test]
    fn self_refers_to_the_declaring_type() {
        let item: Item = parse_quote! {
            #[kaa]
            struct Section {
                #[kaa::refer(Self, "Subsections.")]
                children: Vec<Self>,
            }
        };
        assert_eq!(targets(&item), ["Section"]);
    }

    #[test]
    fn a_type_alias_has_no_self() {
        let item: Item = parse_quote! {
            #[kaa]
            #[kaa::refer(Self, "Sections in reading order.")]
            type Outline = Vec<Section>;
        };
        let error = type_declaration(&item)
            .err()
            .expect("`Self` is rejected")
            .to_string();
        assert_eq!(error, "a type alias has no `Self`");
    }

    #[test]
    fn a_description_reads_as_the_macro_reads_it() {
        let empty: Item = parse_quote! {
            #[kaa()]
            struct Empty;
        };
        let bracketed: Item = parse_quote! {
            #[kaa["A bracketed description."]]
            struct Bracketed;
        };
        assert_eq!(type_declaration(&empty).unwrap().description, None);
        assert_eq!(
            type_declaration(&bracketed).unwrap().description.as_deref(),
            Some("A bracketed description.")
        );
    }

    #[test]
    fn type_items_include_plain_types_and_descend_into_inline_modules() {
        let file: syn::File = parse_quote! {
            #[kaa]
            struct Member;
            struct Plain;
            mod directory {
                #[kaa("A team.")]
                enum Team {}
                fn local() {
                    #[kaa]
                    struct Hidden;
                }
            }
            #[kaalang::kaa]
            type Alias = Member;
        };
        let paths: Vec<String> = type_items(&file.items)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        assert_eq!(paths, ["Member", "Plain", "directory::Team", "Alias"]);
    }
}
