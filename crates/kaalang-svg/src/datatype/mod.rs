//! Renders a type diagram: read each card's type, lay out the cards
//! and their routes, and serialize the scene.

use kaalang_compiler::{Annotation, Field, TypeForm};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Item, Meta, ext::IdentExt, spanned::Spanned};

use crate::{
    RenderError, TypeCard, collapsed, collapsed_text, location, parse_file, parser_source,
    validate_description, validate_label,
};

mod layout;
mod svg;

pub(crate) fn render(
    focus: TypeCard<'_>,
    incoming: &[TypeCard<'_>],
) -> Result<String, RenderError> {
    let incoming = incoming.iter().map(read).collect::<Result<Vec<_>, _>>()?;
    let focus = read(&focus)?;
    Ok(svg::serialize(&layout::layout(&incoming, &focus)))
}

/// What one card shows, in source order. Descriptions stay authored
/// Markdown; source-derived labels are literal.
struct Card {
    /// The caller's name for the type, then its generics as declared.
    name: String,
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
fn read(card: &TypeCard<'_>) -> Result<Card, RenderError> {
    let file = parse_file(card.source)?;
    let source = parser_source(card.source, &file);
    let mut matches = kaalang_compiler::type_items(&file.items)
        .into_iter()
        .filter(|(path, _)| path == card.path);
    let Some((_, item)) = matches.next() else {
        return Err(RenderError::TypeNotFound(card.path.to_owned()));
    };
    if matches.next().is_some() {
        return Err(RenderError::AmbiguousType(card.path.to_owned()));
    }
    let declaration = kaalang_compiler::type_declaration(&item).map_err(|error| {
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
        kind: kind(source, &item),
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
    let mut tiles = |fields: &[Field]| -> Vec<Tile> {
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
                Tile {
                    label: Some(
                        field
                            .name
                            .as_ref()
                            .map_or_else(|| position.to_string(), |name| name.unraw().to_string()),
                    ),
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
        TypeForm::Struct(fields) => Structure::Row(tiles(fields)),
        TypeForm::Union(fields) => Structure::Stack(tiles(fields)),
        TypeForm::Enum(variants) if variants.is_empty() => Structure::Empty,
        TypeForm::Enum(variants) => Structure::Variants(
            variants
                .iter()
                .map(|variant| VariantTile {
                    name: variant.name.unraw().to_string(),
                    case: variant.case.clone(),
                    tiles: tiles(&variant.fields),
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

/// `struct · repr(Rust)`: the form, and the representation Rust's `repr`
/// attributes give it, or its default. A `repr` under `cfg_attr` makes the
/// representation depend on the target, so the form stands alone.
fn kind(source: &str, item: &Item) -> String {
    let (form, attributes) = match item {
        Item::Struct(declaration) => ("struct", &declaration.attrs),
        Item::Enum(declaration) => ("enum", &declaration.attrs),
        Item::Union(declaration) => ("union", &declaration.attrs),
        _ => return "type alias".to_owned(),
    };
    if attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg_attr")
            && matches!(&attribute.meta, Meta::List(list) if mentions_repr(list.tokens.clone()))
    }) {
        return form.to_owned();
    }
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
    let representation = if representations.is_empty() {
        "Rust".to_owned()
    } else {
        representations.join(", ")
    };
    format!("{form} · repr({representation})")
}

/// Whether `repr` appears anywhere in these tokens, nested `cfg_attr` included.
fn mentions_repr(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == "repr",
        TokenTree::Group(group) => mentions_repr(group.stream()),
        _ => false,
    })
}

/// Checks every text the card shows for characters XML 1.0 cannot represent.
fn validate(card: &Card, path: &str, span: Span) -> Result<(), RenderError> {
    let (tiles, cases): (Vec<&Tile>, Vec<&String>) = match &card.structure {
        Structure::Empty => (Vec::new(), Vec::new()),
        Structure::Row(tiles) | Structure::Stack(tiles) => (tiles.iter().collect(), Vec::new()),
        Structure::Variants(variants) => (
            variants.iter().flat_map(|variant| &variant.tiles).collect(),
            variants.iter().flat_map(|variant| &variant.case).collect(),
        ),
    };
    let variants = match &card.structure {
        Structure::Variants(variants) => variants.iter().map(|variant| &variant.name).collect(),
        _ => Vec::new(),
    };
    let labels = [&card.name, &card.kind].into_iter().chain(variants).chain(
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
        .chain(cases)
        .chain(tiles.iter().flat_map(|tile| &tile.description));
    for description in descriptions {
        validate_description(description, span, format!("description of type `{path}`"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Structure, structure};

    /// Whether each tile with a target draws its line, in source order.
    fn drawn(source: &str, drawn: &[usize]) -> Vec<bool> {
        let file = syn::parse_file(source).unwrap();
        let declaration = kaalang_compiler::type_declaration(&file.items[0]).unwrap();
        let tiles = match structure(source, &declaration.form, drawn).0 {
            Structure::Empty => Vec::new(),
            Structure::Row(tiles) | Structure::Stack(tiles) => tiles,
            Structure::Variants(variants) => variants
                .into_iter()
                .flat_map(|variant| variant.tiles)
                .collect(),
        };
        tiles
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
