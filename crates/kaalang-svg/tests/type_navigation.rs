//! What an editor asks of a type diagram: what each path a card's tiles name,
//! each relationship's target, and each incoming card's header stand for.
//!
//! Hotspots are checked against the SVG they describe: each sits on a line of
//! text the diagram draws, so a hotspot that drifts from the drawing fails.

use kaalang_compiler::{SourceFile, SourceType, Target};
use kaalang_svg::{Hotspot, HotspotKind, TypeCard, TypeOptions, render_type};
use kaalang_testing::corpus::{self, Suite};

/// Font size of a tile's type and target.
const FONT: i32 = 12;

/// The number of the first `name="…"` attribute in `text`.
fn number(text: &str, name: &str) -> i32 {
    let (_, value) = text
        .split_once(&format!("{name}=\""))
        .expect("the element has the attribute");
    let (value, _) = value.split_once('"').expect("the attribute is quoted");
    value.parse().expect("a whole number")
}

/// Every line of each `<text>` of `class`, as its `x` and its baseline.
fn lines(svg: &str, class: &str) -> Vec<(i32, i32)> {
    let mut lines = Vec::new();
    for text in svg.split(&format!("<text class=\"{class}\"")).skip(1) {
        let (text, _) = text.split_once("</text>").expect("a text closes");
        let mut baseline = number(text, "y");
        // A styled run nests a `<tspan>` without its own `x`.
        for tspan in text.split("<tspan x=\"").skip(1) {
            let (x, rest) = tspan.split_once('"').expect("the x is quoted");
            baseline += number(rest, "dy");
            lines.push((x.parse().expect("a whole number"), baseline));
        }
    }
    lines
}

/// Calls `check` with each type of `file` drawn as the focus under the other
/// types of the file whose relationships name it bare, as the fixture diagrams
/// draw them: the cards, the focus first, the SVG, and its hotspots.
fn each_diagram(file: &SourceFile<'_>, mut check: impl FnMut(&[&SourceType], &str, &[Hotspot])) {
    let types: Vec<SourceType> = kaalang_compiler::type_items(file.items())
        .iter()
        .filter_map(|(path, _)| file.ty(path).ok())
        .collect();
    let naming = |ty: &SourceType, name: &str| corpus::naming(&ty.declaration, name);
    for focus in &types {
        let name = focus.name.to_string();
        let incoming: Vec<&SourceType> = types
            .iter()
            .filter(|ty| ty.path != focus.path && !naming(ty, &name).is_empty())
            .collect();
        let relationships: Vec<Vec<usize>> = incoming.iter().map(|ty| naming(ty, &name)).collect();
        let cards: Vec<TypeCard> = incoming
            .iter()
            .zip(&relationships)
            .map(|(ty, relationships)| TypeCard {
                ty,
                header: &ty.path,
                relationships,
            })
            .collect();
        let own = naming(focus, &name);
        let card = TypeCard {
            ty: focus,
            header: &focus.path,
            relationships: &own,
        };
        let (svg, hotspots) = render_type(&card, &[&cards], TypeOptions::default())
            .unwrap_or_else(|error| panic!("{}: {error}", focus.path));
        let drawn: Vec<&SourceType> = std::iter::once(focus).chain(incoming).collect();
        check(&drawn, &svg, &hotspots);
    }
}

#[test]
fn every_card_names_its_paths_targets_and_header_where_it_draws_them() {
    let mut checked = 0;
    for (path, source) in corpus::files(Suite::Ordinary) {
        let file = SourceFile::parse(&source, Target::EMPTY).expect("the fixture parses");
        each_diagram(&file, |cards, svg, hotspots| {
            let what = format!("{} in {}", cards[0].path, path.display());
            let (width, height) = (number(svg, "width"), number(svg, "height"));
            let types = lines(svg, "field-type");
            let targets = lines(svg, "target");
            for (number, card) in cards.iter().enumerate() {
                let of = |kind: fn(&HotspotKind) -> Option<usize>| -> Vec<&Hotspot> {
                    hotspots
                        .iter()
                        .filter(|hotspot| kind(&hotspot.kind) == Some(number))
                        .collect()
                };
                let names = of(|kind| match kind {
                    HotspotKind::Path { card } => Some(*card),
                    _ => None,
                });
                let mut spans: Vec<_> = names
                    .iter()
                    .map(|hotspot| file.range(hotspot.span.unwrap()))
                    .collect();
                spans.dedup();
                let expected: Vec<String> = card
                    .field_paths
                    .iter()
                    .zip(&card.field_types)
                    .flat_map(|(names, text)| {
                        names.iter().map(|name| {
                            let path = &text[name.range.clone()];
                            path.rsplit("::").next().unwrap().to_owned()
                        })
                    })
                    .collect();
                let found: Vec<&str> = spans.iter().map(|range| &source[range.clone()]).collect();
                assert_eq!(found, expected, "{what}: card {number} names");
                for hotspot in &names {
                    let baseline = hotspot.bottom - FONT / 2;
                    assert!(
                        types
                            .iter()
                            .any(|&(x, line)| line == baseline && x <= hotspot.left),
                        "{what}: {hotspot:?} is on no type's line"
                    );
                }
                let drawn_targets = of(|kind| match kind {
                    HotspotKind::Target { card } => Some(*card),
                    _ => None,
                });
                assert_eq!(
                    drawn_targets.len(),
                    card.relationship_targets.len(),
                    "{what}: card {number} targets"
                );
                for hotspot in &drawn_targets {
                    let baseline = hotspot.bottom - FONT / 2;
                    assert!(
                        targets.contains(&(hotspot.right, baseline)),
                        "{what}: {hotspot:?} ends no target"
                    );
                }
                let headers = of(|kind| match kind {
                    HotspotKind::Header { card } => Some(*card),
                    _ => None,
                });
                assert_eq!(
                    headers.len(),
                    usize::from(number > 0),
                    "{what}: card {number}"
                );
                for hotspot in &headers {
                    assert_eq!(
                        &source[file.range(hotspot.span.unwrap())],
                        card.name.to_string(),
                        "{what}"
                    );
                }
            }
            for hotspot in hotspots {
                assert!(
                    0 <= hotspot.left
                        && hotspot.left < hotspot.right
                        && hotspot.right <= width
                        && 0 <= hotspot.top
                        && hotspot.top < hotspot.bottom
                        && hotspot.bottom <= height,
                    "{what}: {hotspot:?} leaves the canvas"
                );
            }
            checked += usize::from(cards.len() > 1);
        });
    }
    assert!(checked > 10, "{checked} diagrams with incoming cards");
}

#[test]
fn a_path_that_wraps_has_a_hotspot_at_the_start_of_each_line() {
    let source = "struct Long {\n    field: crate::alpha::beta::gamma::delta::epsilon::zeta::eta::theta::iota::Kappa,\n}\n";
    let file = SourceFile::parse(source, Target::EMPTY).expect("the source parses");
    let ty = file.ty("Long").expect("the type reads");
    let card = TypeCard {
        ty: &ty,
        header: "Long",
        relationships: &[],
    };
    let (svg, hotspots) = render_type(&card, &[], TypeOptions::default()).expect("it renders");
    let lines = lines(&svg, "field-type");
    let paths: Vec<(i32, i32)> = hotspots
        .iter()
        .filter(|hotspot| matches!(hotspot.kind, HotspotKind::Path { .. }))
        .map(|hotspot| (hotspot.left, hotspot.bottom - FONT / 2))
        .collect();
    assert!(lines.len() > 1, "the type wraps: {lines:?}");
    assert_eq!(paths, lines);
}
