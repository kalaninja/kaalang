//! Renders validated kaalang flows and type diagrams as standalone SVG.
//!
//! It serves `cargo kaalang` and the bindings an editor's page loads, and its
//! API may change in any minor release.

use std::{error::Error, fmt, num::NonZeroUsize};

#[cfg(doc)]
use kaalang_compiler::{Analysis, SemanticModel, SourceFile};
use kaalang_compiler::{SourceFlow, SourceType, line_and_column};
use proc_macro2::Span;
use syn::Visibility;

/// Appends one line to the SVG.
macro_rules! emit {
    ($svg:expr, $($argument:tt)*) => {
        writeln!($svg, $($argument)*).expect("writing to a String cannot fail")
    };
}

/// Appends to the SVG without ending the line. Under `xml:space="preserve"` the
/// whitespace between `<text>` and its `<tspan>` children is rendered content,
/// so a label is written as one unbroken line.
macro_rules! emit_inline {
    ($svg:expr, $($argument:tt)*) => {
        write!($svg, $($argument)*).expect("writing to a String cannot fail")
    };
}

mod datatype;
mod flow;
mod markup;
mod text;

/// Presentation options for one rendered type diagram.
#[derive(Clone, Copy, Default)]
pub struct TypeOptions {
    /// How many incoming cards a row holds. Without it, a row holds as many
    /// as fit a Full HD screen, or the focus when it is wider.
    pub columns: Option<NonZeroUsize>,
}

/// An error produced while rendering a kaalang flow or type diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// A verified arrangement could not be realized in pixels. This is a
    /// renderer defect; impossible authored topologies are rejected earlier,
    /// when the flow is read, as [`kaalang_compiler::SourceError::InvalidFlow`].
    UnroutableTopology {
        name: String,
        /// The spatial rule the realized geometry could not meet, naming the
        /// connections, or the label and node, that break it.
        reason: String,
    },
    /// A rendered label contains a character that XML 1.0 cannot represent.
    InvalidLabelCharacter {
        character: char,
        /// The 1-based source line and column of the construct that carries it.
        line: usize,
        column: usize,
        context: String,
    },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnroutableTopology { name, reason } => {
                write!(formatter, "could not route kaalang flow `{name}`: {reason}")
            }
            Self::InvalidLabelCharacter {
                character,
                line,
                column,
                context,
            } => write!(
                formatter,
                "kaalang {context} at {line}:{column} contains XML-incompatible character U+{:04X}",
                *character as u32
            ),
        }
    }
}

impl Error for RenderError {}

/// Renders a flow as it was read from its source file. Returns the SVG and its
/// hotspots: every node that draws a source construct, every drawn cycle
/// boundary, every wire name in a label, and every segment of a connection and
/// every merge, with its ribbon.
///
/// # Errors
///
/// Returns [`RenderError::InvalidLabelCharacter`] when a rendered label
/// contains a character XML 1.0 cannot represent, and
/// [`RenderError::UnroutableTopology`] when realizing the model's checked
/// arrangement cannot route every connection or place every label. A flow is
/// named by its path within the file.
pub fn render_flow(flow: SourceFlow) -> Result<(String, Vec<Hotspot>), RenderError> {
    flow::render(flow)
}

/// What a [`Hotspot`] shows. It states what the drawing stands for and leaves
/// what a click does to the reader.
#[derive(Clone, Debug)]
pub enum HotspotKind {
    /// A node, or a drawn cycle boundary. Its span is the source the node
    /// draws, as [`Analysis::node_span`] places it; a boundary's is its cycle.
    /// A boundary's hotspot holds its body's, so under a point the innermost
    /// hotspot is the one meant, whatever the order of the list.
    Node,
    /// A call node: a node whose call names `callee`, the last segment of the
    /// callee path.
    Call { callee: Span },
    /// One wire name in a label. Its span is where its producer binds the
    /// wire, as [`Analysis::binding`] places it.
    Wire,
    /// A strip centred on one segment of a drawn connection, with its ribbon,
    /// as [`SemanticModel::ribbons`] gives it. It reaches past a corner or a
    /// junction but stops at a node. Strips of shared segments
    /// overlap, and so do those of neighbouring ones. It has no span: no one
    /// place in the source stands for it.
    Connection { ribbon: Vec<RibbonWire> },
    /// The square on a merge, with its ribbon, as [`SemanticModel::ribbons`]
    /// gives it. It overlaps the strips of the connections that meet there,
    /// and has no span.
    Merge { ribbon: Vec<RibbonWire> },
    /// One path a tile's type names, on one line it is drawn on, of the type
    /// `card` draws: 0 for the focus, then the incoming cards in the order
    /// [`render_type`] takes them. Its span is the path's last segment, as
    /// [`SourceType::field_paths`] places it.
    Path { card: usize },
    /// A relationship's target, of the type `card` draws. Its span names the
    /// target's type, as [`SourceType::target_spans`] places it. `focus` marks
    /// a tile whose relationship line reaches the focus.
    Target { card: usize, focus: bool },
    /// The header of an incoming card, the type `card` draws. Its span is the
    /// type's name in its declaration.
    Header { card: usize },
}

/// One wire of a ribbon: its displayed name, and where its producer binds it,
/// as [`Analysis::binding`] places it.
#[derive(Clone, Debug)]
pub struct RibbonWire {
    pub name: String,
    pub binding: Span,
}

/// One element of a rendered diagram that stands for something in its source.
#[derive(Clone, Debug)]
pub struct Hotspot {
    pub kind: HotspotKind,
    /// The element's rectangle in the SVG's user units. A name that wraps has
    /// one hotspot per line.
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// The source the element stands for, a span of the flow or of the type
    /// as it was read, or none for a connection or a merge.
    /// [`SourceFile::range`] gives its bytes, but only while proc-macro2 still
    /// holds the parsed text: it forgets every span when its source map is
    /// cleared.
    pub span: Option<Span>,
    /// For a field-type path, where it stands in the SVG's field-type text,
    /// so that a browser can measure the glyphs it draws in its own font.
    pub field_text: Option<FieldText>,
}

/// Where a path stands on one line of a type diagram's field-type text.
#[derive(Clone, Debug)]
pub struct FieldText {
    /// The line among the SVG's field-type lines.
    pub line: usize,
    /// The text of that line.
    pub text: String,
    /// The path's bytes in that text.
    pub range: std::ops::Range<usize>,
}

/// Reads the flow at `path` within `source` for no target, for tests that
/// name a flow rather than count it.
#[cfg(test)]
pub(crate) fn read_path(
    source: &str,
    path: &str,
    collapse_cycles: bool,
) -> Result<SourceFlow, kaalang_compiler::SourceError> {
    let file = kaalang_compiler::SourceFile::parse(source, kaalang_compiler::CfgSet::EMPTY)?;
    let position = kaalang_compiler::flows(file.items())
        .iter()
        .position(|flow| flow.path == path)
        .unwrap_or_else(|| panic!("the source declares no flow `{path}`"));
    file.flow(position, collapse_cycles)
}

/// Renders the flow at `path` within `source`, which must read.
#[cfg(test)]
pub(crate) fn render_path(
    source: &str,
    path: &str,
    collapse_cycles: bool,
) -> Result<String, RenderError> {
    let flow = read_path(source, path, collapse_cycles)
        .unwrap_or_else(|error| panic!("`{path}` does not read: {error}"));
    render_flow(flow).map(|(svg, _)| svg)
}

/// One card of a type diagram.
#[derive(Clone, Copy)]
pub struct TypeCard<'a> {
    /// The type the card draws, as it was read from the file that declares it.
    pub ty: &'a SourceType,
    /// The name the card shows before the type's generics, such as its path
    /// from its crate root.
    pub header: &'a str,
    /// The relationships drawn as lines into the focus, by their position
    /// among the type's relationships in source order, as the target compiles
    /// it. On the focus card, these are its relationships to itself.
    pub relationships: &'a [usize],
}

/// A card the renderer could not draw, numbered as in its hotspots.
#[derive(Debug)]
pub struct CardError {
    /// Zero for the focus, then the incoming cards in their given order.
    pub card: usize,
    pub error: RenderError,
}

impl fmt::Display for CardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for CardError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// Renders a type diagram: the focus card under the incoming cards, in the
/// given order, with a line for each relationship they draw into the focus.
/// Each group of incoming cards starts a new row and wraps into further rows
/// after `options.columns` cards, or where it would grow wider than a screen,
/// or than the focus when it is wider. Returns the SVG and its hotspots: every
/// path a tile's type names, every relationship's target, and every incoming
/// card's header.
///
/// # Errors
///
/// Returns the first [`CardError`] in drawing order when a card's text
/// contains a character XML 1.0 cannot represent.
///
/// # Panics
///
/// Panics when a card draws a relationship index its type does not have.
pub fn render_type(
    focus: &TypeCard<'_>,
    incoming: &[&[TypeCard<'_>]],
    options: TypeOptions,
) -> Result<(String, Vec<Hotspot>), CardError> {
    datatype::render(focus, incoming, options.columns)
}

/// An incoming card's type for [`card_headers`]: its path from its crate's
/// root, and the name of its crate when that is not the focus's.
#[derive(Clone, Copy, Debug)]
pub struct CardPath<'a> {
    pub path: &'a str,
    pub other_crate: Option<&'a str>,
}

/// The header of each incoming card: a path from the focus's crate drops the
/// module segments it shares with `module`, the focus's module path with a
/// trailing `::`, or nothing at the crate root, and gets them back one by one
/// where it would read as another header or as `focus`, the focus's own; a
/// path from another crate keeps the crate's name and its whole path.
#[must_use]
pub fn card_headers(focus: &str, module: &str, incoming: &[CardPath<'_>]) -> Vec<String> {
    let mut headers: Vec<String> = incoming
        .iter()
        .map(|card| {
            if let Some(krate) = card.other_crate {
                return format!("{krate}::{}", card.path);
            }
            let shared: usize = module
                .split_inclusive("::")
                .zip(card.path.split_inclusive("::"))
                .take_while(|(left, right)| left == right)
                .map(|(segment, _)| segment.len())
                .sum();
            card.path[shared..].to_owned()
        })
        .collect();
    loop {
        let mut seen = std::collections::BTreeSet::from([focus]);
        let collisions: std::collections::BTreeSet<String> = headers
            .iter()
            .filter(|header| !seen.insert(header.as_str()))
            .cloned()
            .collect();
        let mut changed = false;
        for (card, header) in incoming.iter().zip(&mut headers) {
            if card.other_crate.is_some() || !collisions.contains(header.as_str()) {
                continue;
            }
            let prefix = &card.path[..card.path.len() - header.len()];
            let Some(prefix) = prefix.strip_suffix("::") else {
                continue;
            };
            let start = prefix.rfind("::").map_or(0, |at| at + 2);
            card.path[start..].clone_into(header);
            changed = true;
        }
        if !changed {
            return headers;
        }
    }
}

/// The UML visibility marker.
fn visibility_marker(visibility: &Visibility) -> char {
    match visibility {
        Visibility::Public(_) => '+',
        Visibility::Restricted(_) => '~',
        Visibility::Inherited => '-',
    }
}

/// `text` after its visibility marker, as `RichText::marked` reads it.
fn with_visibility(visibility: &Visibility, text: &str) -> String {
    format!("{} {text}", visibility_marker(visibility))
}

fn validate_description(
    label: &str,
    span: Span,
    context: impl Into<String>,
) -> Result<(), RenderError> {
    let context = context.into();
    // Character references decode before serialization, so check both the
    // authored description and the characters a reader sees.
    validate_label(label, span, context.clone())?;
    validate_label(text::RichText::markdown_text(label).as_ref(), span, context)
}

fn validate_label(label: &str, span: Span, context: impl Into<String>) -> Result<(), RenderError> {
    let Some(character) = label
        .chars()
        .find(|character| !valid_xml_character(*character))
    else {
        return Ok(());
    };
    let (line, column) = line_and_column(span);
    Err(RenderError::InvalidLabelCharacter {
        character,
        line,
        column,
        context: context.into(),
    })
}

/// Whether XML 1.0 can represent the character.
const fn valid_xml_character(character: char) -> bool {
    matches!(
        character,
        '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
    )
}

#[cfg(test)]
mod tests {
    use kaalang_compiler::{CfgSet, SourceFile};

    use super::{CardPath, card_headers};

    #[test]
    fn an_incoming_header_drops_the_modules_it_shares_with_the_focus() {
        let card = |path, other_crate| CardPath { path, other_crate };
        assert_eq!(
            card_headers(
                "shop::orders::Order",
                "shop::orders::",
                &[
                    card("shop::orders::Line", None),
                    card("shop::payments::Payment", None),
                    card("shop::orders::Line", Some("external")),
                ],
            ),
            ["Line", "payments::Payment", "external::shop::orders::Line"]
        );
    }

    #[test]
    fn colliding_headers_get_their_modules_back() {
        let card = |path| CardPath {
            path,
            other_crate: None,
        };
        assert_eq!(
            card_headers(
                "shop::orders::Order",
                "shop::orders::",
                &[card("shop::orders::Note"), card("shop::Note")],
            ),
            ["orders::Note", "shop::Note"]
        );
    }

    use super::*;

    /// Renders the type at `path` in `source`, read for no target, alone.
    fn render_card(source: &str, path: &str) -> Result<String, RenderError> {
        let ty = SourceFile::parse(source, CfgSet::EMPTY)
            .unwrap()
            .ty(path)
            .unwrap_or_else(|error| panic!("{error}"));
        let card = TypeCard {
            ty: &ty,
            header: path,
            relationships: &[],
        };
        render_type(&card, &[], TypeOptions::default())
            .map(|(svg, _)| svg)
            .map_err(|error| error.error)
    }

    #[test]
    fn a_plain_type_renders() {
        assert!(
            render_card("struct Plain { id: u32 }\n", "Plain").is_ok_and(|svg| svg.contains("id"))
        );
    }

    #[test]
    fn rejects_type_text_that_xml_cannot_represent() {
        let source = "#[kaa(\"bad\\u{1}text\")]\nstruct Bad;\n";

        assert_eq!(
            render_card(source, "Bad"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\u{1}',
                line: 1,
                column: 1,
                context: "description of type `Bad`".into(),
            })
        );
    }

    #[test]
    fn rejects_a_type_label_that_xml_cannot_represent() {
        let source = "struct Bad {\n    field: Vec</*\0*/ u8>,\n}\n";

        assert_eq!(
            render_card(source, "Bad"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\0',
                line: 1,
                column: 1,
                context: "label of type `Bad`".into(),
            })
        );
    }

    #[test]
    fn a_card_names_its_form_with_its_representations() {
        let source =
            "#[kaa] #[repr(C)] #[repr(align(8))] struct Combined(u8);\nstruct Plain(u8);\n";

        assert!(
            render_card(source, "Combined")
                .unwrap()
                .contains(">struct · repr(C, align(8))</tspan>")
        );
        assert!(
            render_card(source, "Plain")
                .unwrap()
                .contains(">struct</tspan>")
        );
    }

    /// Renders the flow at `path` within `source`, read for no target.
    fn render_source(source: &str, path: &str) -> Result<String, RenderError> {
        render_path(source, path, false)
    }

    #[test]
    fn a_flow_in_a_module_an_impl_or_a_trait_renders() {
        let flow = "fn copy(input: u8) -> u8 {\n    #[kaa::action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";
        let source = format!(
            "mod review {{ #[kaa] {flow} }}\nstruct Report;\nimpl Report {{ #[kaa] {flow} }}\ntrait Review {{ #[kaa] {flow} }}\n"
        );

        for path in ["review::copy", "Report::copy", "Review::copy"] {
            assert!(render_source(&source, path).is_ok(), "{path}");
        }
    }

    #[test]
    fn a_traits_method_carries_no_visibility_marker() {
        let flow = "fn copy(input: u8) -> u8 {\n    #[kaa::action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";
        let source = format!("mod review {{ #[kaa] {flow} }}\ntrait Review {{ #[kaa] {flow} }}\n");
        let badge = "class=\"md-badge\"";
        let svg = render_source(&source, "Review::copy").unwrap();
        assert!(!svg.contains(badge) && svg.contains("Start: copy"), "{svg}");
        assert!(
            render_source(&source, "review::copy")
                .unwrap()
                .contains(badge)
        );
    }

    #[test]
    fn rejects_characters_that_xml_cannot_represent() {
        let source = "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"bad\\0label\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\0',
                line: 3,
                column: 5,
                context: "block description".into(),
            })
        );
    }

    #[test]
    fn rejects_xml_incompatible_signature_labels() {
        assert!("λ\t\n\r<&>".chars().all(valid_xml_character));

        for (source, line, column, context) in [
            (
                "#[kaa]\nfn /*\0*/ invalid(input: u8) -> u8 {\n    |input| return input;\n}\n",
                2,
                1,
                "flow header",
            ),
            (
                "\u{feff}#!/usr/bin/env rustx\r\n#[kaa]\nfn invalid(input: /*\0*/ u8) -> u8 {\n    |input| return input;\n}\n",
                3,
                12,
                "parameter 1",
            ),
            (
                "#[kaa]\nfn invalid(input: u8) -> (u8, /*\0*/ u8) {\n    |input| return input;\n}\n",
                2,
                26,
                "return type",
            ),
        ] {
            assert_eq!(
                render_source(source, "invalid"),
                Err(RenderError::InvalidLabelCharacter {
                    character: '\0',
                    line,
                    column,
                    context: context.into(),
                })
            );
        }
    }

    #[test]
    fn validates_xml_characters_in_default_call_labels() {
        let source = "#[kaa]\nfn invalid() -> usize {\n    #[kaa::call]\n    let output = count::<{r\"a\0b\".len()}>();\n    |output| return output;\n}\nfn count<const N: usize>() -> usize { N }\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\0',
                line: 3,
                column: 5,
                context: "call label".into(),
            })
        );
        let described = source.replace("#[kaa::call]", "#[kaa::call(\"Count bytes.\")]");
        assert!(render_source(&described, "invalid").is_ok());
    }

    #[test]
    fn reports_an_invalid_case_description_by_its_position() {
        let source = "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::choice(\"Pick\")]\n    #[kaa::case(\"first\")]\n    #[kaa::case(\"bad\\0case\")]\n    let (a, b) = |input| {\n        match input { 0 => (), _ => () }\n    };\n\n    #[kaa::action(\"A\")]\n    let end = |a| { 1 };\n\n    #[kaa::action(\"B\")]\n    let end = |b| { 2 };\n\n    |end| return end;\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\0',
                line: 3,
                column: 5,
                context: "case 2 description".into(),
            })
        );
    }

    /// A character reference decodes before the diagram is written, so the
    /// character it names is validated too.
    #[test]
    fn reports_an_invalid_character_a_reference_introduces() {
        let source = "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"Record&#30;the run.\")]\n    let end = |input| { input };\n\n    |end| return end;\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\u{1e}',
                line: 3,
                column: 5,
                context: "block description".into(),
            })
        );
        // Escaping the ampersand keeps the reference literal, and valid.
        let escaped = source.replace("Record&#30;the", r"Record\\&#30;the");
        assert!(render_source(&escaped, "invalid").is_ok());
    }

    #[test]
    fn validates_raw_and_interpreted_stage_descriptions() {
        let source = r#"#[kaa]
fn invalid(go: u8) -> u8 {
    #[kaa::stage("Record&#30;the run.")]
    let finish = |go| {
        #[kaa::action("Forward the value.")]
        let finish = |go| go;
    };
    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}
"#;
        for (source, character) in [
            (source.to_owned(), '\u{1e}'),
            (source.replace("&#30;", r"\0"), '\0'),
        ] {
            assert_eq!(
                render_source(&source, "invalid"),
                Err(RenderError::InvalidLabelCharacter {
                    character,
                    line: 4,
                    column: 19,
                    context: "stage description".into(),
                })
            );
        }
        let escaped = source.replace("Record&#30;the", r"Record\\&#30;the");
        assert!(render_source(&escaped, "invalid").is_ok());
    }

    #[test]
    fn reports_an_invalid_question_branch_description() {
        let source = "#[kaa]\nfn invalid(condition: bool) -> u8 {\n    #[kaa::question(\"Choose\")]\n    #[kaa::yes(\"bad\\0branch\")]\n    #[kaa::no]\n    let (yes, no) = |condition| { condition };\n    #[kaa::action(\"Yes\")]\n    let end = |yes| { 1 };\n    #[kaa::action(\"No\")]\n    let end = |no| { 0 };\n    |end| return end;\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\0',
                line: 3,
                column: 5,
                context: "question branch 1 description".into(),
            })
        );
    }

    #[test]
    fn source_prefixes_preserve_the_diagram_and_label_error_locations() {
        const SOURCE: &str =
            "#[kaa]\nfn idéntity(value: u32) -> u32 {\n    |value| return value;\n}\n";
        const INVALID: &str = "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"bad\\0label\")]\n    let output = |input| input;\n    |output| return output;\n}\n";
        const PREFIXES: [(&str, usize); 4] = [
            ("", 0),
            ("\u{feff}", 0),
            ("#!/usr/bin/env rustx\n", 1),
            ("\u{feff}#!/usr/bin/env rustx\r\n", 1),
        ];

        let reference = render_source(SOURCE, "idéntity").expect("the plain source renders");
        assert!(reference.contains("idéntity"));
        assert!(reference.contains("value: u32"));
        assert!(reference.contains(">u32</tspan>"));
        for (prefix, _) in PREFIXES {
            assert_eq!(
                render_source(&format!("{prefix}{SOURCE}"), "idéntity"),
                Ok(reference.clone())
            );
        }
        assert_eq!(
            render_source(&format!("#![allow(dead_code)]\n{SOURCE}"), "idéntity"),
            Ok(reference)
        );

        let location = |source: &str| match render_source(source, "invalid") {
            Err(RenderError::InvalidLabelCharacter { line, column, .. }) => (line, column),
            other => panic!("expected an invalid label, got {other:?}"),
        };
        let (line, column) = location(INVALID);
        for (prefix, added_lines) in PREFIXES {
            assert_eq!(
                location(&format!("{prefix}{INVALID}")),
                (line + added_lines, column)
            );
        }
    }
}
