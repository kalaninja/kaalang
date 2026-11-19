//! What [`crate::draw_type`] answers: a type diagram's SVG and its hotspots.

use std::collections::BTreeMap;

use kaalang_compiler::{
    SourceError, SourceFile, SourceType,
    workspace::{Inclusion, first_readings},
};
use kaalang_svg::{CardError, CardPath, HotspotKind, TypeCard, TypeOptions};
use serde::{Deserialize, Serialize};

use self::order::{Place, Places};
use crate::utf16;

mod order;

/// Where the editor can ask for the definitions of a file's relationship targets.
pub(crate) fn relationship_targets(text: &str, cfg: &str) -> Result<String, String> {
    let file = SourceFile::parse(text, &cfg.parse()?).map_err(|error| error.to_string())?;
    let offsets = utf16::Offsets::new(text);
    let targets: Vec<_> = file
        .relationship_offsets()
        .into_iter()
        .map(|byte| offsets.at(byte))
        .collect();
    Ok(serde_json::to_string(&targets).expect("source offsets serialize"))
}

/// What the editor found about the focus beyond its own text.
#[derive(Default, Deserialize)]
struct Context {
    #[serde(flatten)]
    focus: References,
    /// Other texts where the engine finds references to the focus.
    #[serde(default)]
    files: Vec<File>,
}

/// What the editor knows about declarations and references in one text.
#[derive(Default, Deserialize)]
struct References {
    /// Configurations of a shared file use the same URI and source offsets.
    uri: Option<String>,
    /// The workspace resolver already distinguished the focus from other
    /// modules that include the same declaration in this target.
    #[serde(default)]
    resolved: bool,
    owner: Option<usize>,
    #[serde(default)]
    module: String,
    #[serde(default)]
    repeated: bool,
    /// Where the engine finds references to the focus in its own text.
    #[serde(default)]
    offsets: Vec<u32>,
    /// The module path of types of the focus's text, by their path within it.
    #[serde(default)]
    modules: BTreeMap<String, String>,
    /// Each type's places in the workspace module tree, by its path in this file.
    #[serde(default)]
    places: Places,
}

/// Another text, with the target's cfg set it is read for.
#[derive(Deserialize)]
struct File {
    text: String,
    cfg: String,
    #[serde(flatten)]
    references: References,
}

/// A text read for its target, with the references in it as bytes.
struct Source<'a> {
    file: SourceFile<'a>,
    offsets: utf16::Offsets,
    references: Vec<usize>,
    editor: &'a References,
}

impl<'a> Source<'a> {
    fn read(text: &'a str, cfg: &str, editor: &'a References) -> Result<Self, String> {
        let file = SourceFile::parse(text, &cfg.parse()?).map_err(|error| error.to_string())?;
        Ok(Self {
            file,
            offsets: utf16::Offsets::new(text),
            references: utf16::bytes(text, &editor.offsets),
            editor,
        })
    }

    fn card(&self, number: usize, ty: &SourceType) -> Card {
        let (start, end) = self.range(ty.name.span());
        Card {
            source: number,
            path: ty.path.clone(),
            start,
            end,
        }
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
    /// Other readings of the same declaration can have earlier places in the
    /// workspace. Locate them before committing to one card.
    #[serde(rename = "otherPlaces", skip_serializing_if = "Vec::is_empty")]
    other_places: Vec<Card>,
}

/// A [`kaalang_svg::Hotspot`] with its span as a UTF-16 range of its source.
#[derive(Serialize)]
struct Spot {
    #[serde(rename = "fieldText", skip_serializing_if = "Option::is_none")]
    field_text: Option<[u32; 3]>,
    kind: &'static str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    focus: bool,
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
struct Incoming<'a> {
    source: usize,
    ty: SourceType,
    relationships: Vec<usize>,
    place: Option<&'a Place>,
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
    let focus_source = Source::read(text, cfg, &context.focus)?;
    let focus = match focus_source.file.ty(path) {
        Err(SourceError::TypeNotFound(_)) => return Ok("null".into()),
        focus => focus.map_err(|error| error.to_string())?,
    };
    // Each source by its number: the focus's text, then each file's.
    let sources: Vec<Option<Source>> = std::iter::once(Some(focus_source))
        .chain(context.files.iter().map(|file| {
            if file.references.offsets.is_empty() {
                return None;
            }
            Source::read(&file.text, &file.cfg, &file.references).ok()
        }))
        .collect();

    let place = Place::first(
        &context.focus.places,
        path,
        None,
        context.focus.modules.get(path).map(String::as_str),
    );
    let (own, mut incoming) = cards(&sources, &focus, place);
    let source = |number: usize| sources[number].as_ref().expect("a card's source is read");
    let modules = |number: usize| &source(number).editor.modules;
    let other_places = keep_first_readings(&focus, &mut incoming, source);
    let (svg, hotspots) = loop {
        match draw(&focus, &own, &incoming, place, modules) {
            Ok(drawn) => break drawn,
            Err(CardError { card: 0, error }) => return Err(error.to_string()),
            Err(error) => {
                incoming.remove(error.card - 1);
            }
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
                HotspotKind::Target { card, .. } => ("target", card),
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
                field_text: hotspot.field_text.as_ref().map(|field| {
                    let offsets = utf16::Offsets::new(&field.text);
                    [
                        u32::try_from(field.line).expect("a diagram has few lines"),
                        offsets.at(field.range.start),
                        offsets.at(field.range.end),
                    ]
                }),
                kind,
                focus: matches!(hotspot.kind, HotspotKind::Target { focus: true, .. }),
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
        .map(|(number, ty)| source(number).card(number, ty))
        .collect();
    Ok(serde_json::to_string(&Drawn {
        svg,
        hotspots,
        cards,
        other_places,
    })
    .expect("a diagram serializes"))
}

/// Renders `focus`, which draws `own`, under `incoming`, with the headers the
/// module paths of each source give.
fn draw<'a>(
    focus: &SourceType,
    own: &[usize],
    incoming: &[Incoming<'a>],
    place: Option<&'a Place>,
    modules: impl Fn(usize) -> &'a BTreeMap<String, String>,
) -> Result<(String, Vec<kaalang_svg::Hotspot>), CardError> {
    let (header, headers) = headers(focus, incoming, place, modules);
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
    let mut groups = Vec::new();
    let mut start = 0;
    for group in incoming.chunk_by(|a, b| {
        a.place.map(|p| p.krate().group_key()) == b.place.map(|p| p.krate().group_key())
    }) {
        let end = start + group.len();
        groups.push(&incoming_cards[start..end]);
        start = end;
    }
    kaalang_svg::render_type(&focus_card, &groups, TypeOptions::default())
}

/// The relationships of `focus`, from source 0, to itself, and every incoming
/// card the references in `sources` reach.
fn cards<'a>(
    sources: &[Option<Source<'a>>],
    focus: &SourceType,
    place: Option<&Place>,
) -> (Vec<usize>, Vec<Incoming<'a>>) {
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
                continue;
            }
            // A confirmed empty list puts this declaration outside every enabled target.
            if place.is_some()
                && source
                    .editor
                    .places
                    .get(&referring)
                    .is_some_and(Vec::is_empty)
            {
                continue;
            }
            if let Ok(ty) = source.file.ty(&referring) {
                incoming.push(Incoming {
                    source: number,
                    ty,
                    relationships,
                    place: Place::first(&source.editor.places, &referring, place, None),
                });
            }
        }
    }
    own.sort_unstable();
    own.dedup();
    if place.is_some() {
        // Unlocated cards follow the known ones in the order the host supplied.
        incoming.sort_by_key(|card| (card.place.is_none(), card.place.map(|p| p.key(place))));
    }
    (own, incoming)
}

/// A shared declaration contributes the first reading in diagram order. The
/// editor still needs the other readings' places to establish that order.
fn keep_first_readings<'a>(
    focus: &SourceType,
    incoming: &mut Vec<Incoming<'_>>,
    source: impl Fn(usize) -> &'a Source<'a>,
) -> Vec<Card> {
    let own = source(0);
    let focus_place = own
        .editor
        .uri
        .as_deref()
        .map(|uri| (uri, own.range(focus.name.span()).0));
    let first = first_readings(incoming.iter().enumerate().filter_map(|(index, card)| {
        let reading = source(card.source);
        let offset = reading.range(card.ty.name.span()).0;
        let uri = reading.editor.uri.as_deref();
        if !reading.editor.resolved && uri.is_some_and(|uri| Some((uri, offset)) == focus_place) {
            return None;
        }
        let inclusion = reading.editor.owner.map(|owner| Inclusion {
            owner,
            module: &reading.editor.module,
            repeated: reading.editor.repeated,
        });
        // Contexts without URIs describe each numbered source only once.
        Some((index, (uri.ok_or(card.source), offset), inclusion))
    }));
    let mut other = Vec::new();
    for (index, card) in std::mem::take(incoming).into_iter().enumerate() {
        if first.contains(&index) {
            incoming.push(card);
        } else {
            other.push(source(card.source).card(card.source, &card.ty));
        }
    }
    other
}

/// The focus's header and each incoming card's: by RFC 0010 when the editor
/// gave every card's module path, else each type's path within its file.
fn headers<'a>(
    focus: &SourceType,
    incoming: &[Incoming<'a>],
    place: Option<&'a Place>,
    modules: impl Fn(usize) -> &'a BTreeMap<String, String>,
) -> (String, Vec<String>) {
    // A module path starts with its crate's name.
    let rooted = |number: usize, ty: &SourceType, place: Option<&'a Place>| {
        let (krate, inner) = if let Some(place) = place {
            (place.krate().name, place.module.as_str())
        } else {
            let module = modules(number).get(&ty.path)?;
            module.split_once("::").unwrap_or((module, ""))
        };
        let name = &ty.name;
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
        .map(|card| rooted(card.source, &card.ty, card.place))
        .collect();
    match (rooted(0, focus, place), paths) {
        (Some((krate, module, header)), Some(paths)) => {
            let cards: Vec<CardPath> = paths
                .iter()
                .zip(incoming)
                .map(|((other, _, path), card)| {
                    let external = match (card.place, place) {
                        (Some(card), Some(focus)) => card.krate() != focus.krate(),
                        _ => *other != krate,
                    };
                    CardPath {
                        path,
                        other_crate: external.then_some(*other),
                    }
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
    fn an_incoming_card_stays_visible_while_its_place_is_unknown() {
        let mut context = json!({
            "places": {"Member": [{"package": "shop", "group": 0, "crate": "shop",
                "root": "src/lib.rs", "module": "model", "order": [0]}]},
            "files": [{"text": REVIEW, "cfg": "", "offsets": [unit(REVIEW, "Member as")]}],
        });
        assert_eq!(cards(&drawn(&context)), ["Member", "Review"]);
        context["files"][0]["places"] = json!({"Review": []});
        assert_eq!(cards(&drawn(&context)), ["Member"]);
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
    fn an_invalid_header_drops_only_its_card_and_preserves_navigation() {
        let drawn = drawn(&json!({
            "offsets": [unit(MODEL, "Member, \"The lead")],
            "modules": {"Member": "shop", "Team": "shop::bad\u{1}"},
            "files": [{
                "text": REVIEW, "cfg": "", "offsets": [unit(REVIEW, "Member as")],
                "modules": {"Review": "other::review"}
            }],
        }));
        assert_eq!(cards(&drawn), ["Member", "Review"]);
        let header = drawn["hotspots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|spot| spot["kind"] == "header")
            .unwrap();
        assert_eq!(header["source"], 1);
        assert_eq!(header["start"], unit(REVIEW, "Review {"));
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
