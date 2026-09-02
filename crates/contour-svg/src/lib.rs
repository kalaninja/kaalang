//! Renders validated Contour flows as standalone SVG diagrams.

use std::{error::Error, fmt};

use proc_macro2::Span;
use syn::{File, Item, ItemFn, Meta};

mod layout;
mod svg;

/// An error produced while selecting, validating, or rendering a Contour flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// The input is not a syntactically valid Rust file.
    Parse {
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// No exact top-level `#[contour]` function has the requested name.
    FlowNotFound(String),
    /// More than one exact top-level `#[contour]` function has the requested name.
    AmbiguousFlow(String),
    /// The selected function is not a valid Contour flow.
    InvalidFlow {
        name: String,
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// An authored label contains a character that XML 1.0 cannot represent.
    InvalidLabelCharacter {
        character: char,
        /// The 1-based source line and column of the block that carries it.
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
            Self::FlowNotFound(name) => write!(
                formatter,
                "top-level `#[contour]` function `{name}` was not found"
            ),
            Self::AmbiguousFlow(name) => write!(
                formatter,
                "multiple top-level `#[contour]` functions are named `{name}`"
            ),
            Self::InvalidFlow {
                name,
                line,
                column,
                message,
            } => write!(
                formatter,
                "invalid Contour flow `{name}` at {line}:{column}: {message}"
            ),
            Self::InvalidLabelCharacter {
                character,
                line,
                column,
                context,
            } => write!(
                formatter,
                "Contour {context} at {line}:{column} contains XML-incompatible character U+{:04X}",
                *character as u32
            ),
        }
    }
}

impl Error for RenderError {}

/// Renders one exact top-level `#[contour]` function from a UTF-8 Rust source file.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust,
/// [`RenderError::FlowNotFound`] or [`RenderError::AmbiguousFlow`] when
/// `flow_name` does not name exactly one top-level `#[contour]` function,
/// [`RenderError::InvalidFlow`] when that function is not a valid Contour flow,
/// and [`RenderError::InvalidLabelCharacter`] when an authored description
/// contains a character XML 1.0 cannot represent.
pub fn render_source(source: &str, flow_name: &str) -> Result<String, RenderError> {
    let file = parse_file(source)?;
    let function = select_flow(&file.items, flow_name)?;
    let graph = contour_model::build(function).map_err(|error| invalid_flow(flow_name, &error))?;
    validate_labels(&graph)?;
    let scene = layout::layout(&graph);

    Ok(svg::serialize(&scene, &graph.name.to_string()))
}

/// Names every top-level `#[contour]` function in a UTF-8 Rust source file, in
/// source order.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust.
pub fn flow_names(source: &str) -> Result<Vec<String>, RenderError> {
    Ok(contour_functions(&parse_file(source)?.items)
        .map(|function| function.sig.ident.to_string())
        .collect())
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

/// Yields the top-level functions carrying a `#[contour]` attribute.
fn contour_functions(items: &[Item]) -> impl Iterator<Item = &ItemFn> {
    items.iter().filter_map(|item| match item {
        Item::Fn(function)
            if function
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("contour")) =>
        {
            Some(function)
        }
        _ => None,
    })
}

fn select_flow<'a>(items: &'a [Item], flow_name: &str) -> Result<&'a ItemFn, RenderError> {
    let mut matches = contour_functions(items).filter(|function| function.sig.ident == flow_name);
    let Some(function) = matches.next() else {
        return Err(RenderError::FlowNotFound(flow_name.to_owned()));
    };
    if matches.next().is_some() {
        return Err(RenderError::AmbiguousFlow(flow_name.to_owned()));
    }
    if let Some(attribute) = function
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("contour"))
        .find(|attribute| match &attribute.meta {
            Meta::Path(_) => false,
            Meta::List(arguments) => !arguments.tokens.is_empty(),
            Meta::NameValue(_) => true,
        })
    {
        return Err(invalid_flow(
            flow_name,
            &syn::Error::new_spanned(attribute, "#[contour] does not accept arguments"),
        ));
    }

    Ok(function)
}

/// Reports a flow-level error at its source position.
fn invalid_flow(flow_name: &str, error: &syn::Error) -> RenderError {
    let (line, column) = location(error.span());
    RenderError::InvalidFlow {
        name: flow_name.to_owned(),
        line,
        column,
        message: error.to_string(),
    }
}

/// Returns the 1-based line and column where a span starts.
fn location(span: Span) -> (usize, usize) {
    let start = span.start();
    (start.line, start.column + 1)
}

fn validate_labels(graph: &contour_model::Graph) -> Result<(), RenderError> {
    for block in &graph.flow.blocks {
        let (line, column) = location(block.span);
        if let Some(character) = block.description.as_deref().and_then(invalid_xml_character) {
            return Err(RenderError::InvalidLabelCharacter {
                character,
                line,
                column,
                context: "block description".to_owned(),
            });
        }
        for (case_index, description) in block.case_descriptions.iter().enumerate() {
            if let Some(character) = invalid_xml_character(description) {
                return Err(RenderError::InvalidLabelCharacter {
                    character,
                    line,
                    column,
                    context: format!("case {} description", case_index + 1),
                });
            }
        }
    }

    Ok(())
}

/// Finds the first character XML 1.0 cannot represent.
fn invalid_xml_character(label: &str) -> Option<char> {
    label.chars().find(|character| {
        !matches!(
            *character,
            '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_an_exact_top_level_contour_attribute() {
        let source = r"
            #[other::contour]
            fn nested_attribute(input: u8) -> u8 {}

            mod inner {
                #[contour]
                fn nested(input: u8) -> u8 {}
            }
        ";

        assert_eq!(
            render_source(source, "nested_attribute"),
            Err(RenderError::FlowNotFound("nested_attribute".into()))
        );
        assert_eq!(
            render_source(source, "nested"),
            Err(RenderError::FlowNotFound("nested".into()))
        );
    }

    #[test]
    fn distinguishes_parse_selection_and_model_errors() {
        assert!(matches!(
            render_source("fn", "broken"),
            Err(RenderError::Parse { .. })
        ));
        assert_eq!(
            render_source("fn plain() {}", "plain"),
            Err(RenderError::FlowNotFound("plain".into()))
        );
        assert!(matches!(
            render_source("#[contour] fn empty() {}", "empty"),
            Err(RenderError::InvalidFlow { .. })
        ));
        assert_eq!(
            render_source(
                "#[contour] fn duplicate() {} #[contour] fn duplicate() {}",
                "duplicate"
            ),
            Err(RenderError::AmbiguousFlow("duplicate".into()))
        );
    }

    #[test]
    fn reports_contour_arguments_as_an_invalid_flow() {
        let source = "#[contour(unexpected)]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Return the input\")]\n    |input| -> result { input };\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidFlow {
                name: "invalid".into(),
                line: 1,
                column: 1,
                message: "#[contour] does not accept arguments".into(),
            })
        );
    }

    #[test]
    fn reports_model_errors_at_their_source_position() {
        let source = "#[contour]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Copy the input\")]\n    |input| -> input { input };\n\n    #[end]\n    |input| {};\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidFlow {
                name: "invalid".into(),
                line: 4,
                column: 16,
                message: "a Contour block output must not reuse a source wire name".into(),
            })
        );
    }

    /// The core model rejects this before `layout` can receive a plan that
    /// repeats the shared consumer.
    #[test]
    fn rejects_a_late_alternative_producer_before_layout() {
        let source = r#"#[contour]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value")]
    |condition| -> (yes, no) { condition };

    #[action("Build the first value")]
    |yes| -> selected { 1 };

    #[action("Use the selected value")]
    |selected| -> result { selected };

    #[action("Build the later alternative")]
    |no| -> selected { 2 };

    #[end]
    |result| {};
}
"#;

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidFlow {
                name: "invalid".into(),
                line: 13,
                column: 13,
                message: "every producer of a Contour wire must be declared before its consumers"
                    .into(),
            })
        );
    }

    #[test]
    fn rejects_characters_that_xml_cannot_represent() {
        let source = "#[contour]\nfn invalid(input: u8) -> u8 {\n    #[action(\"bad\\0label\")]\n    |input| -> result { input };\n\n    #[end]\n    |result| {};\n}\n";

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
    fn reports_an_invalid_case_description_by_its_position() {
        let source = "#[contour]\nfn invalid(input: u8) -> u8 {\n    #[choice(\"Pick\")]\n    #[case(\"first\")]\n    #[case(\"bad\\0case\")]\n    |input| -> (a, b) {\n        match input { 0 => (), _ => () }\n    };\n\n    #[action(\"A\")]\n    |a| -> result { 1 };\n\n    #[action(\"B\")]\n    |b| -> result { 2 };\n\n    #[end]\n    |result| {};\n}\n";

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
}
