//! Renders a type diagram: read each card's type, lay out the cards
//! and their routes, and serialize the scene.

use std::num::NonZeroUsize;

use kaalang_compiler::{Annotation, Field, TypeForm};
use proc_macro2::Span;
use syn::{Item, Meta, Visibility, ext::IdentExt, spanned::Spanned};

use crate::{
    RenderError, Target, TypeCard, collapsed, collapsed_text, location, parse_configured,
    parser_source, validate_description, validate_label, visibility_marker, with_visibility,
};

mod layout;
mod svg;

pub(crate) fn render(
    focus: &TypeCard<'_>,
    incoming: &[&[TypeCard<'_>]],
    columns: Option<NonZeroUsize>,
) -> Result<String, RenderError> {
    // A file that declares several cards is parsed once for each target.
    let mut parsed = Vec::new();
    let groups = incoming
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|card| read(card, &mut parsed))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let groups: Vec<&[Card]> = groups.iter().map(Vec::as_slice).collect();
    let focus = read(focus, &mut parsed)?;
    Ok(svg::serialize(&layout::layout(&groups, &focus, columns)))
}

/// A card's source file, parsed once for every card it declares.
struct Parsed<'a> {
    source: &'a str,
    target: &'a Target,
    /// The source the parser read, without a byte order mark or shebang.
    parser_source: &'a str,
    /// Every type declaration with its path within the file.
    items: Vec<(String, Item)>,
}

/// What one card shows, in source order. Descriptions stay authored
/// Markdown; source-derived labels are literal.
struct Card {
    /// The caller's name for the type, then its generics as declared.
    name: String,
    /// The type's visibility marker, drawn before the name.
    marker: char,
    /// The declaration form and, except on an alias, its representation.
    kind: String,
    description: Option<String>,
    structure: Structure,
}

enum Structure {
    /// A unit or empty struct, or an enum without variants.
    Empty,
    /// A struct's fields, side by side.
    Row(Vec<Tile>),
    Variants(Vec<VariantTile>),
    /// A union's fields, or an alias's right-hand side, one under another.
    Stack(Vec<Tile>),
}

impl Structure {
    /// Every tile, in source order.
    fn tiles(&self) -> Vec<&Tile> {
        match self {
            Self::Empty => Vec::new(),
            Self::Row(tiles) | Self::Stack(tiles) => tiles.iter().collect(),
            Self::Variants(variants) => {
                variants.iter().flat_map(|variant| &variant.tiles).collect()
            }
        }
    }
}

struct VariantTile {
    name: String,
    case: Option<String>,
    tiles: Vec<Tile>,
}

/// One field, or an alias's right-hand side.
struct Tile {
    /// The field's name or position; an alias has none.
    label: Option<String>,
    ty: String,
    description: Option<String>,
    target: Option<String>,
    /// Whether a line leaves this tile for the focus.
    drawn: bool,
}

/// Selects one card's declaration and reads what its card shows.
fn read<'a>(card: &TypeCard<'a>, parsed: &mut Vec<Parsed<'a>>) -> Result<Card, RenderError> {
    let index = if let Some(index) = parsed
        .iter()
        .position(|file| std::ptr::eq(file.source, card.source) && file.target == card.target)
    {
        index
    } else {
        let file = parse_configured(card.source, card.target)?;
        parsed.push(Parsed {
            source: card.source,
            target: card.target,
            parser_source: parser_source(card.source, &file),
            items: kaalang_compiler::type_items(&file.items),
        });
        parsed.len() - 1
    };
    let Parsed {
        parser_source: source,
        items,
        ..
    } = &parsed[index];
    // Only code rustc rejects declares a path twice for one target.
    let Some((_, item)) = items.iter().find(|(path, _)| path == card.path) else {
        return Err(RenderError::TypeNotFound(card.path.to_owned()));
    };
    let declaration = kaalang_compiler::type_declaration(item).map_err(|error| {
        let (line, column) = location(error.span());
        RenderError::InvalidType {
            name: card.path.to_owned(),
            line,
            column,
            message: error.to_string(),
        }
    })?;

    let (structure, relationships) = structure(source, &declaration.form, card.relationships);
    assert!(
        card.relationships
            .iter()
            .all(|index| *index < relationships),
        "`{}` has {relationships} relationships, fewer than the drawn indices {:?} need",
        card.path,
        card.relationships
    );

    let generics = &declaration.generics;
    let generics = match (&generics.lt_token, &generics.gt_token) {
        (Some(open), Some(close)) => {
            collapsed_text(&source[open.span.byte_range().start..close.span.byte_range().end])
        }
        _ => String::new(),
    };
    let read = Card {
        name: format!("{}{generics}", card.header),
        marker: visibility_marker(visibility(item)),
        kind: kind(source, item),
        description: declaration.description,
        structure,
    };
    validate(&read, card.path, item.span())?;
    Ok(read)
}

/// The card's structure, and how many relationships the type has. They count
/// in source order, as `TypeDeclaration::relationships` lists them.
fn structure(source: &str, form: &TypeForm, drawn: &[usize]) -> (Structure, usize) {
    let mut relationships = 0;
    let mut tiles = |fields: &[Field], in_variant: bool| -> Vec<Tile> {
        fields
            .iter()
            .enumerate()
            .map(|(position, field)| {
                let (description, target) = match &field.annotation {
                    None => (None, None),
                    Some(Annotation::Value(description)) => (Some(description.clone()), None),
                    Some(Annotation::Refer(relationship)) => {
                        relationships += 1;
                        (
                            Some(relationship.description.clone()),
                            Some((
                                relationships - 1,
                                collapsed(source, relationship.target.span()),
                            )),
                        )
                    }
                };
                let name = field
                    .name
                    .as_ref()
                    .map_or_else(|| position.to_string(), |name| name.unraw().to_string());
                Tile {
                    // A variant's fields take the enum's visibility.
                    label: Some(if in_variant {
                        name
                    } else {
                        with_visibility(&field.visibility, &name)
                    }),
                    ty: collapsed(source, field.ty.span()),
                    description,
                    drawn: target
                        .as_ref()
                        .is_some_and(|(index, _)| drawn.contains(index)),
                    target: target.map(|(_, target)| target),
                }
            })
            .collect()
    };
    let structure = match form {
        TypeForm::Struct(fields) | TypeForm::Union(fields) if fields.is_empty() => Structure::Empty,
        TypeForm::Struct(fields) => Structure::Row(tiles(fields, false)),
        TypeForm::Union(fields) => Structure::Stack(tiles(fields, false)),
        TypeForm::Enum(variants) if variants.is_empty() => Structure::Empty,
        TypeForm::Enum(variants) => Structure::Variants(
            variants
                .iter()
                .map(|variant| VariantTile {
                    name: variant.name.unraw().to_string(),
                    case: variant.case.clone(),
                    tiles: tiles(&variant.fields, true),
                })
                .collect(),
        ),
        TypeForm::Alias(ty, relationship) => {
            relationships = usize::from(relationship.is_some());
            Structure::Stack(vec![Tile {
                label: None,
                ty: collapsed(source, ty.span()),
                description: relationship.as_ref().map(|it| it.description.clone()),
                target: relationship
                    .as_ref()
                    .map(|it| collapsed(source, it.target.span())),
                drawn: relationship.is_some() && drawn.contains(&0),
            }])
        }
    };
    (structure, relationships)
}

fn visibility(item: &Item) -> &Visibility {
    match item {
        Item::Struct(declaration) => &declaration.vis,
        Item::Enum(declaration) => &declaration.vis,
        Item::Union(declaration) => &declaration.vis,
        Item::Type(declaration) => &declaration.vis,
        _ => unreachable!("`type_items` lists only the supported forms"),
    }
}

/// `struct · repr(C)`: the form, and the representation Rust's `repr`
/// attributes give it, or the form alone for the default. A `repr` under a
/// `cfg_attr` that holds is one of them, since configuring the source expanded
/// it.
fn kind(source: &str, item: &Item) -> String {
    let (form, attributes) = match item {
        Item::Struct(declaration) => ("struct", &declaration.attrs),
        Item::Enum(declaration) => ("enum", &declaration.attrs),
        Item::Union(declaration) => ("union", &declaration.attrs),
        _ => return "type alias".to_owned(),
    };
    let representations: Vec<String> = attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("repr"))
        .filter_map(|attribute| match &attribute.meta {
            Meta::List(list) => {
                let delimiters = list.delimiter.span();
                Some(collapsed_text(
                    &source
                        [delimiters.open().byte_range().end..delimiters.close().byte_range().start],
                ))
            }
            _ => None,
        })
        // An empty `repr()` leaves the default, as Rust reads it.
        .filter(|representation| !representation.is_empty())
        .collect();
    if representations.is_empty() {
        return form.to_owned();
    }
    format!("{form} · repr({})", representations.join(", "))
}

/// Checks every text the card shows for characters XML 1.0 cannot represent.
fn validate(card: &Card, path: &str, span: Span) -> Result<(), RenderError> {
    let tiles = card.structure.tiles();
    let variants: &[VariantTile] = match &card.structure {
        Structure::Variants(variants) => variants,
        _ => &[],
    };
    let names = variants.iter().map(|variant| &variant.name);
    let labels = [&card.name, &card.kind].into_iter().chain(names).chain(
        tiles
            .iter()
            .flat_map(|tile| tile.label.iter().chain([&tile.ty]).chain(&tile.target)),
    );
    for label in labels {
        validate_label(label, span, format!("label of type `{path}`"))?;
    }
    let descriptions = card
        .description
        .iter()
        .chain(variants.iter().flat_map(|variant| &variant.case))
        .chain(tiles.iter().flat_map(|tile| &tile.description));
    for description in descriptions {
        validate_description(description, span, format!("description of type `{path}`"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::structure;

    /// Whether each tile with a target draws its line, in source order.
    fn drawn(source: &str, drawn: &[usize]) -> Vec<bool> {
        let file = syn::parse_file(source).unwrap();
        let declaration = kaalang_compiler::type_declaration(&file.items[0]).unwrap();
        structure(source, &declaration.form, drawn)
            .0
            .tiles()
            .iter()
            .filter(|tile| tile.target.is_some())
            .map(|tile| tile.drawn)
            .collect()
    }

    #[test]
    fn draws_the_relationships_at_the_given_source_positions() {
        let variants = r#"#[kaa] enum Approval {
            Named(#[kaa::refer(Member, "One.")] u8),
            Quorum {
                #[kaa::value("Not a relationship.")]
                votes: u8,
                #[kaa::refer(Member, "Two.")]
                voters: u8,
            },
        }"#;
        let alias = r#"#[kaa] #[kaa::refer(Member, "The member.")] type MemberRef = u8;"#;

        assert_eq!(drawn(variants, &[1]), [false, true]);
        assert_eq!(drawn(alias, &[]), [false]);
        assert_eq!(drawn(alias, &[0]), [true]);
    }
}
