//! Parses type declarations with their annotations, and expands datatypes for
//! the macro.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Error, Generics, Ident, Item, LitStr, Meta, Path, Result, Token, Type, Visibility,
    parse::ParseStream, spanned::Spanned,
};

use crate::{
    is_kaalang_attribute,
    parse::{annotation_name, description},
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
                    let cfgs: Vec<Attribute> = cfg_attributes(&variant.attrs).cloned().collect();
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
/// alike, in source order, each with its path among them: `Member`, or `directory::Team` inside
/// an inline module.
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
/// Returns the first annotation error at its source span.
pub fn expand_type(arguments: &TokenStream, mut item: Item) -> Result<TokenStream> {
    // Spanned at the arguments, so that a malformed description points at them.
    let attribute: Attribute = match arguments.clone().into_iter().next() {
        None => syn::parse_quote!(#[kaa]),
        Some(first) => syn::parse_quote_spanned!(first.span()=> #[kaa(#arguments)]),
    };
    let annotated: Item = syn::parse_quote!(#attribute #item);

    // Only the annotations go: rustc rejects a `#[kaa]` left on a field or variant.
    let unannotated = |attribute: &Attribute| annotation_name(attribute).is_none();
    let mut fields: Vec<&mut syn::Field> = match &mut item {
        Item::Struct(declaration) => {
            declaration.attrs.retain(unannotated);
            declaration.fields.iter_mut().collect()
        }
        Item::Enum(declaration) => {
            declaration.attrs.retain(unannotated);
            declaration
                .variants
                .iter_mut()
                .flat_map(|variant| {
                    variant.attrs.retain(unannotated);
                    variant.fields.iter_mut()
                })
                .collect()
        }
        Item::Union(declaration) => {
            declaration.attrs.retain(unannotated);
            declaration.fields.named.iter_mut().collect()
        }
        Item::Type(declaration) => {
            declaration.attrs.retain(unannotated);
            Vec::new()
        }
        // Rejected before validation, so that the span leaves out the inserted attribute.
        _ => return Err(Error::new_spanned(item, UNSUPPORTED)),
    };
    for field in &mut fields {
        field.attrs.retain(unannotated);
    }
    let declaration = type_declaration(&annotated)?;

    // Each import is spanned at its target, where rustc reports an unknown one,
    // and stands under its field's `cfg`, as the target may.
    let imports: Vec<TokenStream> = declaration
        .relationships()
        .into_iter()
        .map(|relationship| {
            let (target, cfgs) = (&relationship.target, &relationship.cfgs);
            quote_spanned!(target.span()=> #(#cfgs)* #[allow(unused_imports)] use #target as _;)
        })
        .collect();
    let imports = (!imports.is_empty()).then(|| quote! { const _: () = { #(#imports)* }; });
    Ok(quote! { #item #imports })
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
                        let cfgs = outer.iter().chain(cfg_attributes(&field.attrs));
                        let cfgs = cfgs.cloned().collect();
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

fn cfg_attributes(attributes: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
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
