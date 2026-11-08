//! Parses annotated type declarations and expands them for the macro.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Error, Generics, Ident, Item, LitStr, Meta, Path, Result, Token, Type,
    parse::ParseStream, spanned::Spanned,
};

use crate::{
    is_kaalang_attribute,
    parse::{description, is_annotation},
};

/// A struct, enum, union, or type alias marked `#[kaa]`.
pub struct TypeDeclaration {
    pub name: Ident,
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
    pub ty: Type,
    pub annotation: Option<Annotation>,
}

pub enum Annotation {
    Value(String),
    Refer(Relationship),
}

pub struct Relationship {
    /// The target as written, a name or a path without parameters.
    pub target: Path,
    pub description: String,
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

const UNSUPPORTED: &str = "#[kaa] applies to functions, structs, enums, unions, and type aliases";

/// Parses an annotated type declaration. Its first `#[kaa]` attribute
/// supplies the optional description.
///
/// # Errors
///
/// Returns the first annotation error at its source span.
pub fn type_declaration(item: &Item) -> Result<TypeDeclaration> {
    let (attributes, name, generics) = match item {
        Item::Struct(declaration) => (
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        ),
        Item::Enum(declaration) => (
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        ),
        Item::Union(declaration) => (
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        ),
        Item::Type(declaration) => (
            &declaration.attrs,
            &declaration.ident,
            &declaration.generics,
        ),
        _ => return Err(Error::new_spanned(item, UNSUPPORTED)),
    };
    let mut kaalang = attributes
        .iter()
        .filter(|attribute| is_kaalang_attribute(attribute));
    let description = match kaalang.next() {
        Some(attribute) if !matches!(attribute.meta, Meta::Path(_)) => {
            Some(description(attribute, "kaalang type")?)
        }
        _ => None,
    };
    if let Some(duplicate) = kaalang.next() {
        return Err(Error::new_spanned(
            duplicate,
            "a type takes one `#[kaa]` attribute",
        ));
    }

    let mut alias_relationship = None;
    for attribute in attributes {
        match annotation(attribute) {
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
            Some("refer") => alias_relationship = Some(relationship(attribute, generics)?),
            _ => {}
        }
    }

    let form = match item {
        Item::Struct(declaration) => TypeForm::Struct(fields(&declaration.fields, generics)?),
        Item::Enum(declaration) => TypeForm::Enum(
            declaration
                .variants
                .iter()
                .map(|variant| {
                    Ok(Variant {
                        name: variant.ident.clone(),
                        case: variant_case(&variant.attrs)?,
                        fields: fields(&variant.fields, generics)?,
                    })
                })
                .collect::<Result<_>>()?,
        ),
        Item::Union(declaration) => TypeForm::Union(fields(&declaration.fields.named, generics)?),
        Item::Type(declaration) => TypeForm::Alias(declaration.ty.clone(), alias_relationship),
        _ => unreachable!("unsupported forms were rejected above"),
    };
    Ok(TypeDeclaration {
        name: name.clone(),
        generics: generics.clone(),
        description,
        form,
    })
}

/// Expands an annotated type for the macro: the declaration without its
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
    let unannotated = |attribute: &Attribute| annotation(attribute).is_none();
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

    // Each import is spanned at its target, where rustc reports an unknown one.
    let imports: Vec<TokenStream> = declaration
        .relationships()
        .into_iter()
        .map(|relationship| {
            let target = &relationship.target;
            quote_spanned!(target.span()=> #[allow(unused_imports)] use #target as _;)
        })
        .collect();
    let imports = (!imports.is_empty()).then(|| quote! { const _: () = { #(#imports)* }; });
    Ok(quote! { #item #imports })
}

/// The name of a type annotation kaalang consumes: `kaa::value`, `kaa::refer`,
/// or `kaa::case`.
fn annotation(attribute: &Attribute) -> Option<&'static str> {
    ["value", "refer", "case"]
        .into_iter()
        .find(|name| is_annotation(attribute, name))
}

fn fields<'a>(
    fields: impl IntoIterator<Item = &'a syn::Field>,
    generics: &Generics,
) -> Result<Vec<Field>> {
    fields
        .into_iter()
        .map(|field| {
            let mut annotation = None;
            for attribute in &field.attrs {
                let parsed = match self::annotation(attribute) {
                    Some("value") => Annotation::Value(description(attribute, "kaalang value")?),
                    Some("refer") => Annotation::Refer(relationship(attribute, generics)?),
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
                ty: field.ty.clone(),
                annotation,
            })
        })
        .collect()
}

fn variant_case(attributes: &[Attribute]) -> Result<Option<String>> {
    let mut case = None;
    for attribute in attributes {
        match annotation(attribute) {
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

/// Parses `refer(Target, "description")`.
fn relationship(attribute: &Attribute, generics: &Generics) -> Result<Relationship> {
    let malformed = || {
        Error::new_spanned(
            attribute,
            "a relationship is written `#[kaa::refer(Target, \"description\")]`",
        )
    };
    let Meta::List(list) = &attribute.meta else {
        return Err(malformed());
    };
    let (target, description) = list
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
    })
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{Item, parse_quote};

    use super::{TypeForm, expand_type, type_declaration};

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
}
