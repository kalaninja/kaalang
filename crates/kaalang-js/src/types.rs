//! What [`crate::draw_type`] answers: a type diagram's SVG and its hotspots.

use std::collections::BTreeMap;

use kaalang_compiler::{SourceError, SourceFile, SourceType};
use kaalang_svg::{CardPath, HotspotKind, TypeCard, TypeOptions};
use serde::{Deserialize, Serialize};

use crate::utf16;

/// What the editor found about the focus beyond its own text.
#[derive(Default, Deserialize)]
struct Context {
    /// Where the engine finds references to the focus in its own text.
    #[serde(default)]
    offsets: Vec<u32>,
    /// The module path of types of the focus's text, by their path within it.
    #[serde(default)]
    modules: BTreeMap<String, String>,
    /// Other texts where the engine finds references to the focus.
    #[serde(default)]
    files: Vec<File>,
}

/// Another text, with the target's cfg set it is read for.
#[derive(Deserialize)]
struct File {
    text: String,
    cfg: String,
    offsets: Vec<u32>,
    #[serde(default)]
    modules: BTreeMap<String, String>,
}

/// A text read for its target, with the references in it as bytes.
struct Source<'a> {
    file: SourceFile<'a>,
    offsets: utf16::Offsets,
    references: Vec<usize>,
    modules: &'a BTreeMap<String, String>,
}

impl<'a> Source<'a> {
    fn read(
        text: &'a str,
        cfg: &str,
        offsets: &[u32],
        modules: &'a BTreeMap<String, String>,
    ) -> Result<Self, String> {
        let file = SourceFile::parse(text, &cfg.parse()?).map_err(|error| error.to_string())?;
        Ok(Self {
            file,
            offsets: utf16::Offsets::new(text),
            references: utf16::bytes(text, offsets),
            modules,
        })
    }

    /// The UTF-16 range of `span`, a span of this text.
    fn range(&self, span: proc_macro2::Span) -> (u32, u32) {
        let range = self.file.range(span);
        (self.offsets.at(range.start), self.offsets.at(range.end))
    }
}

/// The answer.
#[derive(Serialize)]
struct Drawn {
    svg: String,
    hotspots: Vec<Spot>,
    /// Every card, the focus first, then the incoming ones in drawing order.
    cards: Vec<Card>,
}

/// A [`kaalang_svg::Hotspot`] with its span as a UTF-16 range of its source.
#[derive(Serialize)]
struct Spot {
    kind: &'static str,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    source: usize,
    start: u32,
    end: u32,
}

/// One card's type: its source, its path within it, and its name's range.
#[derive(Serialize)]
struct Card {
    source: usize,
    path: String,
    start: u32,
    end: u32,
}

/// One incoming card before it is drawn.
struct Incoming {
    source: usize,
    ty: SourceType,
    relationships: Vec<usize>,
}

/// The answer as JSON, `null` when no type has `path` in `text`, or why the
/// diagram cannot be drawn. A text of `context` that cannot be read, or a type
/// of one that is not valid, draws no card.
pub(crate) fn ty(text: &str, cfg: &str, path: &str, context: &str) -> Result<String, String> {
    let context: Context = if context.is_empty() {
        Context::default()
    } else {
        serde_json::from_str(context).map_err(|error| error.to_string())?
    };
    let focus_source = Source::read(text, cfg, &context.offsets, &context.modules)?;
    let focus = match focus_source.file.ty(path) {
        Err(SourceError::TypeNotFound(_)) => return Ok("null".into()),
        focus => focus.map_err(|error| error.to_string())?,
    };
    // Each source by its number: the focus's text, then each file's.
    let sources: Vec<Option<Source>> = std::iter::once(Some(focus_source))
        .chain(
            context
                .files
                .iter()
                .map(|file| Source::read(&file.text, &file.cfg, &file.offsets, &file.modules).ok()),
        )
        .collect();

    let (own, mut incoming) = cards(&sources, &focus);
    let source = |number: usize| sources[number].as_ref().expect("a card's source is read");
    let modules = |number: usize| source(number).modules;
    let (svg, hotspots) = match draw(&focus, &own, &incoming, modules) {
        Ok(drawn) => drawn,
        Err(error) => {
            // A card whose own text the renderer rejects is left out; the
            // focus's error stands.
            incoming.retain(|card| {
                let alone = TypeCard {
                    ty: &card.ty,
                    header: &card.ty.path,
                    relationships: &[],
                };
                kaalang_svg::render_type(&alone, &[], TypeOptions::default()).is_ok()
            });
            draw(&focus, &own, &incoming, modules).map_err(|_| error)?
        }
    };

    // Card 0 is the focus, from source 0; card n the incoming card n - 1.
    let card_source = |card: usize| {
        card.checked_sub(1)
            .map_or(0, |index| incoming[index].source)
    };
    let hotspots = hotspots
        .iter()
        .map(|hotspot| {
            let (kind, card) = match hotspot.kind {
                HotspotKind::Path { card } => ("path", card),
                HotspotKind::Target { card } => ("target", card),
                HotspotKind::Header { card } => ("header", card),
                HotspotKind::Node
                | HotspotKind::Call { .. }
                | HotspotKind::Wire
                | HotspotKind::Connection { .. }
                | HotspotKind::Merge { .. } => unreachable!("a type diagram draws no flow"),
            };
            let number = card_source(card);
            let (start, end) =
                source(number).range(hotspot.span.expect("a card's hotspot has a span"));
            Spot {
                kind,
                left: hotspot.left,
                top: hotspot.top,
                right: hotspot.right,
                bottom: hotspot.bottom,
                source: number,
                start,
                end,
            }
        })
        .collect();
    let cards = std::iter::once((0, &focus))
        .chain(incoming.iter().map(|card| (card.source, &card.ty)))
        .map(|(number, ty)| {
            let (start, end) = source(number).range(ty.name.span());
            Card {
                source: number,
                path: ty.path.clone(),
                start,
                end,
            }
        })
        .collect();
    Ok(serde_json::to_string(&Drawn {
        svg,
        hotspots,
        cards,
    })
    .expect("a diagram serializes"))
}

/// Renders `focus`, which draws `own`, under `incoming`, with the headers the
/// module paths of each source give.
fn draw<'a>(
    focus: &SourceType,
    own: &[usize],
    incoming: &[Incoming],
    modules: impl Fn(usize) -> &'a BTreeMap<String, String>,
) -> Result<(String, Vec<kaalang_svg::Hotspot>), String> {
    let (header, headers) = headers(focus, incoming, modules);
    let focus_card = TypeCard {
        ty: focus,
        header: &header,
        relationships: own,
    };
    let incoming_cards: Vec<TypeCard> = incoming
        .iter()
        .zip(&headers)
        .map(|(card, header)| TypeCard {
            ty: &card.ty,
            header,
            relationships: &card.relationships,
        })
        .collect();
    kaalang_svg::render_type(&focus_card, &[&incoming_cards], TypeOptions::default())
        .map_err(|error| error.to_string())
}

/// The relationships of `focus`, from source 0, to itself, and every incoming
/// card the references in `sources` reach.
fn cards(sources: &[Option<Source>], focus: &SourceType) -> (Vec<usize>, Vec<Incoming>) {
    // A `Self` target is always the focus's own.
    let mut own: Vec<usize> = (0..)
        .zip(&focus.relationship_targets)
        .filter(|(_, target)| *target == "Self")
        .map(|(position, _)| position)
        .collect();
    let mut incoming = Vec::new();
    for (number, source) in sources.iter().enumerate() {
        let Some(source) = source else { continue };
        for (referring, relationships) in source.file.referring(&source.references) {
            if number == 0 && referring == focus.path {
                own.extend(relationships);
            } else if let Ok(ty) = source.file.ty(&referring) {
                incoming.push(Incoming {
                    source: number,
                    ty,
                    relationships,
                });
            }
        }
    }
    own.sort_unstable();
    own.dedup();
    (own, incoming)
}

/// The focus's header and each incoming card's: by RFC 0010 when the editor
/// gave every card's module path, else each type's path within its file.
fn headers<'a>(
    focus: &SourceType,
    incoming: &[Incoming],
    modules: impl Fn(usize) -> &'a BTreeMap<String, String>,
) -> (String, Vec<String>) {
    // A module path starts with its crate's name.
    let rooted = |number: usize, ty: &SourceType| {
        let module = modules(number).get(&ty.path)?;
        let name = &ty.name;
        let (krate, inner) = module.split_once("::").unwrap_or((module, ""));
        let module = if inner.is_empty() {
            String::new()
        } else {
            format!("{inner}::")
        };
        let path = format!("{module}{name}");
        Some((krate, module, path))
    };
    let paths: Option<Vec<(&str, String, String)>> = incoming
        .iter()
        .map(|card| rooted(card.source, &card.ty))
        .collect();
    match (rooted(0, focus), paths) {
        (Some((krate, module, header)), Some(paths)) => {
            let cards: Vec<CardPath> = paths
                .iter()
                .map(|(other, _, path)| CardPath {
                    path,
                    other_crate: (*other != krate).then_some(*other),
                })
                .collect();
            let headers = kaalang_svg::card_headers(&header, &module, &cards);
            (header, headers)
        }
        _ => (
            focus.path.clone(),
            incoming.iter().map(|card| card.ty.path.clone()).collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::ty;
    use crate::utf16;

    const MODEL: &str = r#"// Модель 🦀
#[kaa]
pub struct Member {
    #[kaa::refer(Self, "The previous one.")]
    previous: Option<Box<Member>>,
}

#[kaa]
pub struct Team {
    #[kaa::refer(Member, "The lead.")]
    lead: u8,
}
"#;

    const REVIEW: &str = r#"use crate::model::Member as Person;

#[kaa]
pub struct Review {
    #[kaa::refer(Person, "The author.")]
    author: u8,
}
"#;

    /// The UTF-16 offset of the first `marker` in `text`.
    fn unit(text: &str, marker: &str) -> u32 {
        utf16::Offsets::new(text).at(text.find(marker).unwrap())
    }

    fn drawn(context: &Value) -> Value {
        serde_json::from_str(&ty(MODEL, "", "Member", &context.to_string()).unwrap()).unwrap()
    }

    /// The paths of the cards of `drawn`, the focus first.
    fn cards(drawn: &Value) -> Vec<&str> {
        drawn["cards"]
            .as_array()
            .unwrap()
            .iter()
            .map(|card| card["path"].as_str().unwrap())
            .collect()
    }

    #[test]
    fn the_focus_draws_alone_without_references() {
        let drawn: Value = serde_json::from_str(&ty(MODEL, "", "Member", "").unwrap()).unwrap();
        assert!(drawn["svg"].as_str().unwrap().starts_with("<svg "));
        assert_eq!(cards(&drawn), ["Member"]);
        assert_eq!(drawn["cards"][0]["start"], unit(MODEL, "Member {"));
    }

    #[test]
    fn references_draw_the_types_that_refer_to_the_focus() {
        let drawn = drawn(&json!({
            "offsets": [unit(MODEL, "Member, \"The lead")],
            "files": [{"text": REVIEW, "cfg": "", "offsets": [unit(REVIEW, "Member as")]}],
        }));
        assert_eq!(cards(&drawn), ["Member", "Team", "Review"]);
        assert_eq!(drawn["cards"][2]["source"], 1);
        assert_eq!(drawn["cards"][2]["start"], unit(REVIEW, "Review {"));
    }

    #[test]
    fn a_name_in_a_field_type_counts_in_its_own_text() {
        let drawn = drawn(&json!({}));
        let boxed = drawn["hotspots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|spot| spot["kind"] == "path" && spot["start"] == unit(MODEL, "Box"))
            .expect("Box has a hotspot");
        assert_eq!(boxed["end"], unit(MODEL, "Box") + 3);
        assert_eq!(boxed["source"], 0);
    }

    #[test]
    fn a_text_that_does_not_read_draws_no_card() {
        let drawn = drawn(&json!({
            "files": [{"text": "struct", "cfg": "", "offsets": [0]}],
        }));
        assert_eq!(cards(&drawn), ["Member"]);
    }

    #[test]
    fn an_incoming_card_that_cannot_be_drawn_is_left_out() {
        let bad = "#[kaa]\npub struct Bad {\n    #[kaa::refer(Member, \"A \u{1} control.\")]\n    lead: u8,\n}\n";
        let drawn = drawn(&json!({
            "offsets": [unit(MODEL, "Member, \"The lead")],
            "files": [{"text": bad, "cfg": "", "offsets": [unit(bad, "Member")]}],
        }));
        assert_eq!(cards(&drawn), ["Member", "Team"]);
    }

    #[test]
    fn module_paths_give_headers_by_rfc_0010() {
        let drawn = drawn(&json!({
            "offsets": [unit(MODEL, "Member, \"The lead")],
            "modules": {"Member": "shop::model", "Team": "shop::model"},
            "files": [{
                "text": REVIEW,
                "cfg": "",
                "offsets": [unit(REVIEW, "Member as")],
                "modules": {"Review": "other::review"},
            }],
        }));
        let svg = drawn["svg"].as_str().unwrap();
        assert!(svg.contains("model::Member"), "{svg}");
        assert!(svg.contains("other::review::Review"), "{svg}");
        assert!(!svg.contains("model::Team"), "{svg}");
    }

    #[test]
    fn no_type_at_the_path_draws_nothing() {
        assert_eq!(ty(MODEL, "", "Missing", ""), Ok("null".into()));
    }
}
