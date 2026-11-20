//! What [`crate::draw_type`] answers: a type diagram's SVG and its hotspots.

use kaalang_compiler::{SourceError, SourceFile, SourceType};
use kaalang_render::CardPath;
use kaalang_svg::{CardError, HotspotKind, TypeCard, TypeOptions};
use serde::{Deserialize, Serialize};

pub(crate) use self::order::{Place, Target};
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

/// What the editor knows about one text.
#[derive(Default, Deserialize)]
struct References {
    /// Where the engine finds references to the focus in the text.
    #[serde(default)]
    offsets: Vec<u32>,
    /// Where the text stands in the workspace's module tree, when it is known.
    place: Option<Place>,
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

    /// Where `ty` of this text stands, when the editor placed the text.
    fn place_of(&self, ty: &SourceType) -> Option<Place> {
        let offset = self.range(ty.name.span()).0;
        self.editor
            .place
            .as_ref()
            .map(|place| place.of_type(&ty.path, offset))
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
struct Incoming {
    source: usize,
    ty: SourceType,
    relationships: Vec<usize>,
    place: Option<Place>,
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

    let source = |number: usize| sources[number].as_ref().expect("a card's source is read");
    let place = source(0).place_of(&focus);
    let (own, mut incoming) = cards(&sources, &focus, place.as_ref());
    let (svg, hotspots) = loop {
        match draw(&focus, &own, &incoming, place.as_ref()) {
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
    })
    .expect("a diagram serializes"))
}

/// Renders `focus`, which draws `own`, at `place` under `incoming`.
fn draw(
    focus: &SourceType,
    own: &[usize],
    incoming: &[Incoming],
    place: Option<&Place>,
) -> Result<(String, Vec<kaalang_svg::Hotspot>), CardError> {
    let (header, headers) = headers(focus, incoming, place);
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
    let groups: Vec<_> = incoming
        .iter()
        .map(|card| card.place.as_ref().map(|place| place.krate().group_key()))
        .collect();
    kaalang_svg::render_type(
        &focus_card,
        &incoming_cards,
        &groups,
        TypeOptions::default(),
    )
}

/// The relationships of `focus`, from source 0, to itself, and every incoming
/// card the references in `sources` reach.
fn cards(
    sources: &[Option<Source<'_>>],
    focus: &SourceType,
    place: Option<&Place>,
) -> (Vec<usize>, Vec<Incoming>) {
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
            if let Ok(ty) = source.file.ty(&referring) {
                incoming.push(Incoming {
                    source: number,
                    place: source.place_of(&ty),
                    ty,
                    relationships,
                });
            }
        }
    }
    own.sort_unstable();
    own.dedup();
    if place.is_some() {
        // Unlocated cards follow the known ones in the order the host supplied.
        incoming.sort_by(|a, b| {
            (a.place.is_none(), a.place.as_ref().map(|p| p.key(place)))
                .cmp(&(b.place.is_none(), b.place.as_ref().map(|p| p.key(place))))
        });
    }
    (own, incoming)
}

/// The focus's header and each incoming card's: by RFC 0010 when the editor
/// placed every card's text, else each type's path within its file.
fn headers(
    focus: &SourceType,
    incoming: &[Incoming],
    place: Option<&Place>,
) -> (String, Vec<String>) {
    // The path from the crate root, and the module it is in.
    let rooted = |ty: &SourceType, place: &Place| {
        let module = if place.module.is_empty() {
            String::new()
        } else {
            format!("{}::", place.module)
        };
        let path = format!("{module}{}", ty.name);
        (module, path)
    };
    let placed: Option<Vec<&Place>> = incoming.iter().map(|card| card.place.as_ref()).collect();
    match (place, placed) {
        (Some(place), Some(placed)) => {
            let (module, header) = rooted(focus, place);
            let paths: Vec<String> = incoming
                .iter()
                .zip(&placed)
                .map(|(card, other)| rooted(&card.ty, other).1)
                .collect();
            let cards: Vec<CardPath> = paths
                .iter()
                .zip(&placed)
                .map(|(path, other)| CardPath {
                    path,
                    other_crate: (other.krate() != place.krate()).then_some(other.krate().name),
                })
                .collect();
            let headers = kaalang_render::card_headers(&header, &module, &cards);
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

    /// `module` of crate `krate`, at source position 0 of its root.
    fn place(krate: &str, module: &str) -> Value {
        json!({"package": krate, "group": 0, "crate": krate, "root": "src/lib.rs",
            "module": module, "order": [0]})
    }

    #[test]
    fn an_incoming_card_stays_visible_while_its_place_is_unknown() {
        let context = json!({
            "place": place("shop", "model"),
            "files": [{"text": REVIEW, "cfg": "", "offsets": [unit(REVIEW, "Member as")]}],
        });
        assert_eq!(cards(&drawn(&context)), ["Member", "Review"]);
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
            "place": place("shop", ""),
            "files": [{
                "text": REVIEW, "cfg": "", "offsets": [unit(REVIEW, "Member as")],
                "place": place("other", "bad\u{1}"),
            }],
        }));
        assert_eq!(cards(&drawn), ["Member", "Team"]);
        let header = drawn["hotspots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|spot| spot["kind"] == "header")
            .unwrap();
        assert_eq!(header["source"], 0);
        assert_eq!(header["start"], unit(MODEL, "Team {"));
    }

    #[test]
    fn module_paths_give_headers_by_rfc_0010() {
        let drawn = drawn(&json!({
            "offsets": [unit(MODEL, "Member, \"The lead")],
            "place": place("shop", "model"),
            "files": [{
                "text": REVIEW,
                "cfg": "",
                "offsets": [unit(REVIEW, "Member as")],
                "place": place("other", "review"),
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
