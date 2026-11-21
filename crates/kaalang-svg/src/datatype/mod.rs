use std::num::NonZeroUsize;

use kaalang_compiler::{Annotation, Field, NamedPath, SourceField, SourceType, TypeForm};
use proc_macro2::Span;
use syn::ext::IdentExt;

use crate::{
    CardError, Hotspot, RenderError, TypeCard, validate_description, validate_label,
    visibility_marker, with_visibility,
};

mod layout;
mod navigation;
mod svg;

pub(crate) fn render<K: PartialEq>(
    focus: &TypeCard<'_>,
    incoming: &[TypeCard<'_>],
    groups: &[K],
    columns: Option<NonZeroUsize>,
) -> Result<(String, Vec<Hotspot>), CardError> {
    let mut position = 0;
    let mut read_card = |card: &TypeCard<'_>| {
        let result = read(card).map_err(|error| CardError {
            card: position,
            error,
        });
        position += 1;
        result
    };
    let focus = read_card(focus)?;
    let incoming = incoming
        .iter()
        .map(&mut read_card)
        .collect::<Result<Vec<_>, _>>()?;
    let scene = layout::layout(&incoming, groups, &focus, columns);
    let incoming: Vec<&Card> = incoming.iter().collect();
    let hotspots = navigation::collect(&scene, &focus, &incoming);
    Ok((svg::serialize(&scene), hotspots))
}

/// What one card shows, in source order. Descriptions stay authored
/// Markdown; source-derived labels are literal.
struct Card {
    /// The caller's name for the type, then its generics as declared.
    name: String,
    /// The type's own name in its declaration.
    name_span: Span,
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
    /// The tile's position among its card's, in source order.
    index: usize,
    /// The field's name or position; an alias has none.
    label: Option<String>,
    ty: String,
    /// The paths `ty` names.
    paths: Vec<NamedPath>,
    description: Option<String>,
    target: Option<String>,
    /// Where the target names its type.
    target_span: Option<Span>,
    /// Whether a line leaves this tile for the focus.
    drawn: bool,
}

fn read(card: &TypeCard<'_>) -> Result<Card, RenderError> {
    let ty = card.ty;
    let structure = structure(ty, card.relationships);
    let relationships = ty.relationships.len();
    assert!(
        card.relationships
            .iter()
            .all(|index| *index < relationships),
        "`{}` has {relationships} relationships, fewer than the drawn indices {:?} need",
        ty.path,
        card.relationships
    );

    let read = Card {
        name: format!("{}{}", card.header, ty.generics),
        name_span: ty.name.span(),
        marker: visibility_marker(&ty.visibility),
        kind: kind(ty),
        description: ty.declaration.description.clone(),
        structure,
    };
    validate(&read, &ty.path, ty.span)?;
    Ok(read)
}

/// The card's structure: each tile with the field the compiler read for it.
fn structure(ty: &SourceType, drawn: &[usize]) -> Structure {
    let tile = |index, label, field: &SourceField, description| {
        let relationship = field.relationship.map(|at| (at, &ty.relationships[at]));
        Tile {
            index,
            label,
            ty: field.ty.clone(),
            paths: field.paths.clone(),
            description,
            drawn: relationship.is_some_and(|(at, _)| drawn.contains(&at)),
            target: relationship.map(|(_, relationship)| relationship.target.clone()),
            target_span: relationship.map(|(_, relationship)| relationship.target_span),
        }
    };
    let mut read = ty.fields.iter().enumerate();
    let mut tiles = |fields: &[Field], in_variant: bool| -> Vec<Tile> {
        fields
            .iter()
            .enumerate()
            .map(|(position, field)| {
                let (index, read) = read.next().expect("each field has its authored type");
                let description = match &field.annotation {
                    None => None,
                    Some(Annotation::Value(description)) => Some(description.clone()),
                    Some(Annotation::Refer(relationship)) => Some(relationship.description.clone()),
                };
                let name = field
                    .name
                    .as_ref()
                    .map_or_else(|| position.to_string(), |name| name.unraw().to_string());
                // A variant's fields take the enum's visibility.
                let label = if in_variant {
                    name
                } else {
                    with_visibility(&field.visibility, &name)
                };
                tile(index, Some(label), read, description)
            })
            .collect()
    };
    match &ty.declaration.form {
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
        TypeForm::Alias(_, relationship) => {
            let description = relationship.as_ref().map(|it| it.description.clone());
            Structure::Stack(vec![tile(0, None, &ty.fields[0], description)])
        }
    }
}

/// `struct · repr(C)`: the form, and the representation Rust's `repr`
/// attributes give it, or the form alone for the default.
fn kind(ty: &SourceType) -> String {
    let form = match &ty.declaration.form {
        TypeForm::Struct(_) => "struct",
        TypeForm::Enum(_) => "enum",
        TypeForm::Union(_) => "union",
        TypeForm::Alias(..) => return "type alias".to_owned(),
    };
    if ty.representations.is_empty() {
        return form.to_owned();
    }
    format!("{form} · repr({})", ty.representations.join(", "))
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
    use kaalang_compiler::{CfgSet, SourceFile};

    use super::structure;

    /// Whether each tile with a target draws its line, in source order.
    fn drawn(source: &str, path: &str, drawn: &[usize]) -> Vec<bool> {
        let ty = SourceFile::parse(source, CfgSet::EMPTY)
            .unwrap()
            .ty(path)
            .unwrap();
        structure(&ty, drawn)
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

        assert_eq!(drawn(variants, "Approval", &[1]), [false, true]);
        assert_eq!(drawn(alias, "MemberRef", &[]), [false]);
        assert_eq!(drawn(alias, "MemberRef", &[0]), [true]);
    }
}
