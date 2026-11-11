//! Renders validated kaalang flows and type diagrams as standalone SVG.
//!
//! It serves `cargo kaalang`, and its API may change in any minor release.

use std::{error::Error, fmt, num::NonZeroUsize};

use proc_macro2::Span;
use syn::{File, Generics, Visibility, spanned::Spanned};

pub use self::target::Target;

/// Appends one line to the SVG. Writing to a `String` cannot fail.
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
mod target;
mod text;

/// Options for one rendered flow: its presentation, and the compilation target
/// its file is read for.
#[derive(Clone, Copy)]
pub struct RenderOptions<'a> {
    /// Replace every validated cycle region with one described cycle node.
    pub collapse_cycles: bool,
    /// The compilation target the file is read for. The default set is empty.
    pub target: &'a Target,
}

impl Default for RenderOptions<'_> {
    fn default() -> Self {
        Self {
            collapse_cycles: false,
            target: Target::EMPTY,
        }
    }
}

/// Presentation options for one rendered type diagram.
#[derive(Clone, Copy, Default)]
pub struct TypeOptions {
    /// How many incoming cards a row holds. Without it, a row holds as many
    /// as fit a Full HD screen, or the focus when it is wider.
    pub columns: Option<NonZeroUsize>,
}

/// An error produced while selecting, validating, or rendering a kaalang flow
/// or type diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// The input is not a syntactically valid Rust file.
    Parse {
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// No `#[kaa]` function stands at the requested position, or has the path
    /// a caller requested.
    FlowNotFound(String),
    /// More than one `#[kaa]` function has the path a caller requested.
    AmbiguousFlow(String),
    /// The selected function is not a valid kaalang flow.
    InvalidFlow {
        name: String,
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// A verified arrangement could not be realized in pixels. This is a renderer
    /// defect; impossible authored topologies are rejected earlier as [`Self::InvalidFlow`].
    UnroutableTopology {
        name: String,
        /// The spatial rule the realized geometry could not meet, naming the
        /// connections, or the label and node, that break it.
        reason: String,
    },
    /// No type has the requested path.
    TypeNotFound(String),
    /// A card's type has annotations that are not valid.
    InvalidType {
        name: String,
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
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
            Self::Parse {
                line,
                column,
                message,
            } => write!(
                formatter,
                "could not parse Rust source at {line}:{column}: {message}"
            ),
            Self::FlowNotFound(name) => {
                write!(formatter, "`#[kaa]` function `{name}` was not found")
            }
            Self::AmbiguousFlow(name) => write!(
                formatter,
                "multiple `#[kaa]` functions have the path `{name}`"
            ),
            Self::InvalidFlow {
                name,
                line,
                column,
                message,
            } => write!(
                formatter,
                "invalid kaalang flow `{name}` at {line}:{column}: {message}"
            ),
            Self::UnroutableTopology { name, reason } => {
                write!(formatter, "could not route kaalang flow `{name}`: {reason}")
            }
            Self::TypeNotFound(name) => {
                write!(formatter, "type `{name}` was not found")
            }
            Self::InvalidType {
                name,
                line,
                column,
                message,
            } => write!(
                formatter,
                "invalid type `{name}` at {line}:{column}: {message}"
            ),
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

/// Renders the flow at `position` among those [`flow_names`] lists for
/// `options.target`, which the caller selected by its own rules.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust,
/// [`RenderError::FlowNotFound`] when no flow stands at `position`,
/// [`RenderError::InvalidFlow`] when that function is not a valid kaalang flow,
/// [`RenderError::InvalidLabelCharacter`] when a rendered label contains a
/// character XML 1.0 cannot represent, and
/// [`RenderError::UnroutableTopology`] when realizing the model's checked
/// arrangement cannot route every connection or place every label. A flow is
/// named by its path within the file.
pub fn render_flow(
    source: &str,
    position: usize,
    options: RenderOptions<'_>,
) -> Result<String, RenderError> {
    flow::render(source, position, options)
}

/// Renders the flow at `path` within `source`, as [`flow_names`] lists it, for
/// tests that name a flow rather than count it.
#[cfg(test)]
pub(crate) fn render_path(
    source: &str,
    path: &str,
    options: RenderOptions<'_>,
) -> Result<String, RenderError> {
    let position = flow_names(source, options.target)?
        .iter()
        .position(|listed| listed == path)
        .ok_or_else(|| RenderError::FlowNotFound(path.to_owned()))?;
    render_flow(source, position, options)
}

/// Lists the path within a UTF-8 Rust source file of every `#[kaa]`
/// function `target` compiles, free or associated, in source order and with
/// duplicates: `route`, `review::route` inside an inline module,
/// `Report::approve`, `<Report as Approve>::approve`, or `Approve::approve`.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust.
pub fn flow_names(source: &str, target: &Target) -> Result<Vec<String>, RenderError> {
    Ok(
        kaalang_compiler::flows(&parse_configured(source, target)?.items)
            .into_iter()
            .map(|flow| flow.path)
            .collect(),
    )
}

/// One card of a type diagram: a type, the name its header shows,
/// and its relationships drawn as lines into the focus.
#[derive(Clone, Copy, Debug)]
pub struct TypeCard<'a> {
    /// The UTF-8 Rust source file that declares the type.
    pub source: &'a str,
    /// The type's path within that file, as `kaalang_compiler::type_items`
    /// lists it.
    pub path: &'a str,
    /// The compilation target the file is read for.
    pub target: &'a Target,
    /// The name the card shows before the type's generics, such as its path
    /// from its crate root.
    pub header: &'a str,
    /// The relationships drawn as lines into the focus, by their position
    /// among the type's relationships in source order, as the target compiles
    /// it. On the focus card, these are its relationships to itself.
    pub relationships: &'a [usize],
}

/// Renders a type diagram: the focus card under the incoming cards, in the
/// given order, with a line for each relationship they draw into the focus.
/// Each group of incoming cards starts a new row and wraps into further rows
/// after `options.columns` cards, or where it would grow wider than a screen,
/// or than the focus when it is wider.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when a card's source is not valid Rust,
/// [`RenderError::TypeNotFound`] when a card's path names no type its target
/// compiles, [`RenderError::InvalidType`] when that type's annotations are not
/// valid, and [`RenderError::InvalidLabelCharacter`] when a card's text
/// contains a character XML 1.0 cannot represent.
///
/// # Panics
///
/// Panics when a card draws a relationship index its type does not have.
pub fn render_type(
    focus: &TypeCard<'_>,
    incoming: &[&[TypeCard<'_>]],
    options: TypeOptions,
) -> Result<String, RenderError> {
    datatype::render(focus, incoming, options.columns)
}

/// Parses `source` and removes what `target` does not compile.
fn parse_configured(source: &str, target: &Target) -> Result<File, RenderError> {
    let mut file = parse_file(source)?;
    target.configure(&mut file);
    Ok(file)
}

fn parse_file(source: &str) -> Result<File, RenderError> {
    syn::parse_file(source).map_err(|error| {
        let (line, column) = location(error.span());
        RenderError::Parse {
            line,
            column,
            message: error.to_string(),
        }
    })
}

/// The exact source suffix `syn::parse_file` tokenized. Its retained newline
/// after a shebang keeps span line numbers aligned with the original file.
fn parser_source<'a>(source: &'a str, file: &File) -> &'a str {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    file.shebang
        .as_ref()
        .map_or(source, |shebang| &source[shebang.len()..])
}

/// The authored text under a span with every whitespace run collapsed to one space.
fn collapsed(source: &str, span: Span) -> String {
    collapsed_text(&source[span.byte_range()])
}

/// The generic parameters a declaration keeps for its target, each as
/// authored, between angle brackets, or nothing when it keeps none.
fn generics_text(source: &str, generics: &Generics) -> String {
    if generics.params.is_empty() {
        return String::new();
    }
    let parameters: Vec<String> = generics
        .params
        .iter()
        .map(|parameter| collapsed(source, parameter.span()))
        .collect();
    format!("<{}>", parameters.join(", "))
}

/// The UML visibility marker: `+` for `pub`, `~` for a restricted form such as
/// `pub(crate)`, and `-` for private.
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

fn collapsed_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Returns the 1-based line and column where a span starts.
fn location(span: Span) -> (usize, usize) {
    let start = span.start();
    (start.line, start.column + 1)
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
    let (line, column) = location(span);
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
    use super::*;

    /// A card of the type at `path` in `source`, read for `target`.
    fn card<'a>(source: &'a str, path: &'a str, target: &'a Target) -> TypeCard<'a> {
        TypeCard {
            source,
            path,
            target,
            header: path,
            relationships: &[],
        }
    }

    #[test]
    fn selects_one_type_by_its_path_annotated_or_not() {
        let source = "mod model { #[kaa(\"\")] struct Empty; }\nstruct Plain { id: u32 }\n";
        let card = |path| card(source, path, Target::EMPTY);

        assert_eq!(
            render_type(&card("Missing"), &[], TypeOptions::default()),
            Err(RenderError::TypeNotFound("Missing".into()))
        );
        assert!(matches!(
            render_type(&card("model::Empty"), &[], TypeOptions::default()),
            Err(RenderError::InvalidType { name, line: 1, .. }) if name == "model::Empty"
        ));
        assert!(
            render_type(&card("Plain"), &[], TypeOptions::default())
                .is_ok_and(|svg| svg.contains("id"))
        );
    }

    #[test]
    fn a_card_reads_its_type_as_its_target_compiles_it() {
        let source = "#[cfg(unix)]\nstruct Handle(u8, #[cfg(windows)] isize);\n#[cfg(not(unix))]\nstruct Handle(u16);\n#[cfg(windows)]\nstruct Raw;\n";
        let unix: Target = "unix".parse().unwrap();
        let svg =
            |target| render_type(&card(source, "Handle", target), &[], TypeOptions::default());

        let read = svg(&unix).unwrap();
        assert!(read.contains(">u8</tspan>"), "{read}");
        assert!(!read.contains("isize"), "{read}");
        assert!(read.contains(">struct</tspan>"), "{read}");
        let other = svg(Target::EMPTY).unwrap();
        assert!(other.contains(">u16</tspan>"), "{other}");
        assert_eq!(
            render_type(&card(source, "Raw", &unix), &[], TypeOptions::default()),
            Err(RenderError::TypeNotFound("Raw".into()))
        );
    }

    #[test]
    fn rejects_type_text_that_xml_cannot_represent() {
        let source = "#[kaa(\"bad\\u{1}text\")]\nstruct Bad;\n";

        assert_eq!(
            render_type(
                &card(source, "Bad", Target::EMPTY),
                &[],
                TypeOptions::default()
            ),
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
            render_type(
                &card(source, "Bad", Target::EMPTY),
                &[],
                TypeOptions::default()
            ),
            Err(RenderError::InvalidLabelCharacter {
                character: '\0',
                line: 1,
                column: 1,
                context: "label of type `Bad`".into(),
            })
        );
    }

    #[test]
    fn a_representation_under_a_cfg_attr_counts_when_it_holds() {
        let source = "#[kaa] #[repr(C)] #[repr(align(8))] struct Combined(u8);\n#[kaa] #[cfg_attr(unix, cfg_attr(target_pointer_width = \"64\", repr(C)))] struct Conditional(u8);\n";
        let unix: Target = "unix\ntarget_pointer_width=\"64\"".parse().unwrap();
        let svg = |path, target| {
            render_type(&card(source, path, target), &[], TypeOptions::default()).unwrap()
        };

        assert!(svg("Combined", Target::EMPTY).contains(">struct · repr(C, align(8))</tspan>"));
        assert!(svg("Conditional", &unix).contains(">struct · repr(C)</tspan>"));
        assert!(svg("Conditional", Target::EMPTY).contains(">struct</tspan>"));
    }

    #[test]
    fn a_card_lists_the_generic_parameters_its_target_compiles() {
        let source = "struct Handle<#[cfg(windows)] W, #[cfg(unix)] U>(u8);\n";
        let unix: Target = "unix".parse().unwrap();
        let svg = |target| {
            render_type(&card(source, "Handle", target), &[], TypeOptions::default()).unwrap()
        };

        assert!(svg(&unix).contains(" Handle&lt;U&gt;</text>"));
        assert!(svg(Target::EMPTY).contains(" Handle</text>"));
    }

    #[test]
    fn a_flow_lists_the_parameters_its_target_compiles() {
        let source = "#[kaa]\nfn copy<#[cfg(windows)] W, #[cfg(unix)] U>(#[cfg(windows)] raw: W, #[cfg(unix)] input: U) -> U {\n    #[kaa::action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";
        let unix: Target = "unix".parse().unwrap();
        let options = RenderOptions {
            target: &unix,
            ..RenderOptions::default()
        };

        let svg = render_path(source, "copy", options).unwrap();
        assert!(svg.contains("copy&lt;U&gt;"), "{svg}");
        assert!(svg.contains("input: U"), "{svg}");
        assert!(!svg.contains("raw") && !svg.contains("cfg"), "{svg}");
    }

    /// Renders the flow at `path` within `source`, read for no target.
    fn render_source(source: &str, path: &str) -> Result<String, RenderError> {
        render_path(source, path, RenderOptions::default())
    }

    #[test]
    fn requires_an_exact_kaalang_attribute() {
        let source = "#[other::kaa]\nfn nested_attribute(input: u8) -> u8 {}\n";

        assert_eq!(flow_names(source, Target::EMPTY), Ok(Vec::new()));
    }

    #[test]
    fn distinguishes_parse_position_and_model_errors() {
        assert!(matches!(
            render_flow("fn", 0, RenderOptions::default()),
            Err(RenderError::Parse { .. })
        ));
        assert_eq!(
            render_flow("fn plain() {}", 0, RenderOptions::default()),
            Err(RenderError::FlowNotFound("0".into()))
        );
        assert!(matches!(
            render_flow("#[kaa] fn empty() {}", 0, RenderOptions::default()),
            Err(RenderError::InvalidFlow { .. })
        ));
    }

    #[test]
    fn finds_a_flow_marked_through_the_crate_path() {
        let source = "#[kaalang::kaa]\nfn qualified(input: u8) -> u8 {\n    #[kaa::action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";

        assert_eq!(
            flow_names(source, Target::EMPTY),
            Ok(vec!["qualified".to_owned()])
        );
        assert!(render_flow(source, 0, RenderOptions::default()).is_ok());
    }

    #[test]
    fn flows_are_listed_by_their_path_within_the_file() {
        let flow = "fn copy(input: u8) -> u8 {\n    #[kaa::action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";
        let source = format!(
            "mod review {{ #[kaa] {flow} }}\nstruct Report;\nimpl Report {{ #[kaa] {flow} }}\ntrait Review {{ #[kaa] {flow} }}\n"
        );

        let paths = ["review::copy", "Report::copy", "Review::copy"];
        assert_eq!(
            flow_names(&source, Target::EMPTY),
            Ok(paths.map(str::to_owned).to_vec())
        );
        for path in paths {
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
    fn reports_kaa_arguments_as_an_invalid_flow() {
        for attribute in ["kaa", "kaalang::kaa"] {
            let source = format!(
                "#[{attribute}(unexpected)]\nfn invalid(input: u8) -> u8 {{\n    #[kaa::action(\"Return the input\")]\n    let end = |input| {{ input }};\n    |end| return end;\n}}\n"
            );

            assert_eq!(
                render_source(&source, "invalid"),
                Err(RenderError::InvalidFlow {
                    name: "invalid".into(),
                    line: 1,
                    column: 1,
                    message: "#[kaa] on a function does not accept arguments".into(),
                })
            );
        }
    }

    #[test]
    fn reports_model_errors_at_their_source_position() {
        let source = "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"Copy the input\")]\n    let input = |input| { input };\n    |input| return input;\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidFlow {
                name: "invalid".into(),
                line: 4,
                column: 9,
                message: "a kaalang block output must not reuse a flow input name".into(),
            })
        );
    }

    /// Language validation rejects this before `layout` can receive a plan that
    /// repeats the shared consumer.
    #[test]
    fn rejects_a_late_alternative_producer_before_layout() {
        let source = r#"#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Choose a value")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build the first value")]
    let selected = |yes| { 1 };

    #[kaa::action("Use the selected value")]
    let end = |selected| { selected };

    #[kaa::action("Build the later alternative")]
    let selected = |no| { 2 };

    |end| return end;
}
"#;

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidFlow {
                name: "invalid".into(),
                line: 13,
                column: 9,
                message: "every producer of a kaalang wire must be declared before its consumers"
                    .into(),
            })
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
    fn source_prefixes_preserve_labels_and_error_locations() {
        const SOURCE: &str =
            "#[kaa]\nfn idéntity(value: u32) -> u32 {\n    |value| return value;\n}\n";
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

        let invalid = [
            ("fn invalid(", "invalid"),
            (
                "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"Copy\")]\n    let input = |input| input;\n    |input| return input;\n}\n",
                "invalid",
            ),
            (
                "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"bad\\0label\")]\n    let output = |input| input;\n    |output| return output;\n}\n",
                "invalid",
            ),
        ];
        let location = |error: &RenderError| match error {
            RenderError::Parse { line, column, .. }
            | RenderError::InvalidFlow { line, column, .. }
            | RenderError::InvalidLabelCharacter { line, column, .. } => (*line, *column),
            other => panic!("expected a source-spanned error, got {other:?}"),
        };
        for (source, flow) in invalid {
            let expected = render_source(source, flow).expect_err("the plain source is invalid");
            let (line, column) = location(&expected);
            for (prefix, added_lines) in PREFIXES {
                let error = render_source(&format!("{prefix}{source}"), flow)
                    .expect_err("the prefixed source stays invalid");
                assert_eq!(location(&error), (line + added_lines, column));
                assert_eq!(
                    std::mem::discriminant(&error),
                    std::mem::discriminant(&expected)
                );
            }
        }
    }
}
