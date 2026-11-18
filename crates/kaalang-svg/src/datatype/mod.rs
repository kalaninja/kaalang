use std::num::NonZeroUsize;

use kaalang_compiler::{Annotation, Field, NamedPath, SourceType, TypeForm};
use proc_macro2::Span;
use syn::{Item, Visibility, ext::IdentExt, spanned::Spanned};

use crate::{
    Hotspot, RenderError, TypeCard, validate_description, validate_label, visibility_marker,
    with_visibility,
};

mod layout;
mod navigation;
mod svg;

pub(crate) fn render(
    focus: &TypeCard<'_>,
    incoming: &[&[TypeCard<'_>]],
    columns: Option<NonZeroUsize>,
) -> Result<(String, Vec<Hotspot>), RenderError> {
    // The focus first: an error names the first card that fails, in the
    // order a caller lists them.
    let focus = read(focus)?;
    let groups = incoming
        .iter()
        .map(|group| group.iter().map(read).collect::<Result<Vec<_>, _>>())
        .collect::<Result<Vec<_>, _>>()?;
    let groups: Vec<&[Card]> = groups.iter().map(Vec::as_slice).collect();
    let scene = layout::layout(&groups, &focus, columns);
    let incoming: Vec<&Card> = groups.iter().flat_map(|group| group.iter()).collect();
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

/// Reads what one card shows of its type.
fn read(card: &TypeCard<'_>) -> Result<Card, RenderError> {
    let ty = card.ty;
    let (structure, relationships) = structure(ty, card.relationships);
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
        marker: visibility_marker(visibility(&ty.item)),
        kind: kind(ty),
        description: ty.declaration.description.clone(),
        structure,
    };
    validate(&read, &ty.path, ty.item.span())?;
    Ok(read)
}

/// The card's structure, and how many relationships the type has. They count
/// in source order, as `TypeDeclaration::relationships` lists them.
fn structure(ty: &SourceType, drawn: &[usize]) -> (Structure, usize) {
    let mut relationships = 0;
    let mut types = ty
        .field_types
        .iter()
        .cloned()
        .zip(ty.field_paths.iter().cloned());
    let mut index = 0;
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
                                ty.relationship_targets[relationships - 1].clone(),
                            )),
                        )
                    }
                };
                let name = field
                    .name
                    .as_ref()
                    .map_or_else(|| position.to_string(), |name| name.unraw().to_string());
                let (field_type, paths) = types.next().expect("each field has its authored type");
                index += 1;
                Tile {
                    index: index - 1,
                    // A variant's fields take the enum's visibility.
                    label: Some(if in_variant {
                        name
                    } else {
                        with_visibility(&field.visibility, &name)
                    }),
                    ty: field_type,
                    paths,
                    description,
                    drawn: target
                        .as_ref()
                        .is_some_and(|(index, _)| drawn.contains(index)),
                    target_span: target.as_ref().map(|(index, _)| ty.target_spans[*index]),
                    target: target.map(|(_, target)| target),
                }
            })
            .collect()
    };
    let structure = match &ty.declaration.form {
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
            relationships = usize::from(relationship.is_some());
            Structure::Stack(vec![Tile {
                index: 0,
                label: None,
                ty: ty.field_types[0].clone(),
                paths: ty.field_paths[0].clone(),
                description: relationship.as_ref().map(|it| it.description.clone()),
                target: ty.relationship_targets.first().cloned(),
                target_span: ty.target_spans.first().copied(),
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
        _ => unreachable!("only the supported forms read as a type"),
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
    use kaalang_compiler::{SourceFile, Target};

    use super::structure;

    /// Whether each tile with a target draws its line, in source order.
    fn drawn(source: &str, path: &str, drawn: &[usize]) -> Vec<bool> {
        let ty = SourceFile::parse(source, Target::EMPTY)
            .unwrap()
            .ty(path)
            .unwrap();
        structure(&ty, drawn)
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

        assert_eq!(drawn(variants, "Approval", &[1]), [false, true]);
        assert_eq!(drawn(alias, "MemberRef", &[]), [false]);
        assert_eq!(drawn(alias, "MemberRef", &[0]), [true]);
    }
}
