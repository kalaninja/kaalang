//! Renders validated kaalang flows as standalone SVG diagrams.

use std::{error::Error, fmt};

use proc_macro2::Span;
use syn::{File, Item, ItemFn, Meta};

mod layout;
mod svg;
mod topology;

/// An error produced while selecting, validating, or rendering a kaalang flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// The input is not a syntactically valid Rust file.
    Parse {
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// No exact top-level `#[kaalang]` function has the requested name.
    FlowNotFound(String),
    /// More than one exact top-level `#[kaalang]` function has the requested name.
    AmbiguousFlow(String),
    /// The selected function is not a valid kaalang flow.
    InvalidFlow {
        name: String,
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// The flow is valid, but this layout could not place it under RFC 0002 §8:
    /// either a connection route or a wire label breaks the spatial contract.
    /// This does not make the authored flow invalid or prove that no conforming
    /// diagram exists.
    UnroutableTopology {
        name: String,
        /// The spatial rule the deterministic layout could not meet, naming the
        /// connections, or the label and node, that break it.
        reason: String,
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
                "top-level `#[kaalang]` function `{name}` was not found"
            ),
            Self::AmbiguousFlow(name) => write!(
                formatter,
                "multiple top-level `#[kaalang]` functions are named `{name}`"
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

/// Renders one exact top-level `#[kaalang]` function from a UTF-8 Rust source file.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust,
/// [`RenderError::FlowNotFound`] or [`RenderError::AmbiguousFlow`] when
/// `flow_name` does not name exactly one top-level `#[kaalang]` function,
/// [`RenderError::InvalidFlow`] when that function is not a valid kaalang flow,
/// [`RenderError::InvalidLabelCharacter`] when an authored description
/// contains a character XML 1.0 cannot represent, and
/// [`RenderError::UnroutableTopology`] when the deterministic layout cannot
/// route every connection and place every label under RFC 0002 §8.
pub fn render_source(source: &str, flow_name: &str) -> Result<String, RenderError> {
    let file = parse_file(source)?;
    let function = select_flow(&file.items, flow_name)?;
    let model = kaalang_model::build(function).map_err(|error| invalid_flow(flow_name, &error))?;
    validate_labels(&model)?;
    let start = layout::start_text(source, &function.sig);
    let parameters = layout::parameter_text(source, &function.sig);
    let return_type = layout::return_text(source, &function.sig.output);
    let scene = layout::layout(&model, &start, &parameters, &return_type).map_err(|reason| {
        RenderError::UnroutableTopology {
            name: flow_name.to_owned(),
            reason,
        }
    })?;

    Ok(svg::serialize(&scene, &model.name.to_string()))
}

/// Names every top-level `#[kaalang]` function in a UTF-8 Rust source file, in
/// source order.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust.
pub fn flow_names(source: &str) -> Result<Vec<String>, RenderError> {
    Ok(kaalang_functions(&parse_file(source)?.items)
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

/// Yields the top-level functions carrying a `#[kaalang]` attribute.
fn kaalang_functions(items: &[Item]) -> impl Iterator<Item = &ItemFn> {
    items.iter().filter_map(|item| match item {
        Item::Fn(function)
            if function
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("kaalang")) =>
        {
            Some(function)
        }
        _ => None,
    })
}

fn select_flow<'a>(items: &'a [Item], flow_name: &str) -> Result<&'a ItemFn, RenderError> {
    let mut matches = kaalang_functions(items).filter(|function| function.sig.ident == flow_name);
    let Some(function) = matches.next() else {
        return Err(RenderError::FlowNotFound(flow_name.to_owned()));
    };
    if matches.next().is_some() {
        return Err(RenderError::AmbiguousFlow(flow_name.to_owned()));
    }
    if let Some(attribute) = function
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("kaalang"))
        .find(|attribute| match &attribute.meta {
            Meta::Path(_) => false,
            Meta::List(arguments) => !arguments.tokens.is_empty(),
            Meta::NameValue(_) => true,
        })
    {
        return Err(invalid_flow(
            flow_name,
            &syn::Error::new_spanned(attribute, "#[kaalang] does not accept arguments"),
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

fn validate_labels(model: &kaalang_model::SemanticModel) -> Result<(), RenderError> {
    for block in &model.flow.blocks {
        let (line, column) = location(block.span);
        if let Some(character) = block.description.as_deref().and_then(invalid_xml_character) {
            return Err(RenderError::InvalidLabelCharacter {
                character,
                line,
                column,
                context: "block description".to_owned(),
            });
        }
        // Only a choice is projected with case nodes, so only a choice reaches
        // the diagram with case labels; the parser leaves this list empty for
        // every other kind.
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
        for (branch, description) in block
            .question_branches
            .iter()
            .enumerate()
            .filter_map(|(branch, answer)| answer.description.as_deref().map(|text| (branch, text)))
        {
            if let Some(character) = invalid_xml_character(description) {
                return Err(RenderError::InvalidLabelCharacter {
                    character,
                    line,
                    column,
                    context: format!("question branch {} description", branch + 1),
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
    fn requires_an_exact_top_level_kaalang_attribute() {
        let source = r"
            #[other::kaalang]
            fn nested_attribute(input: u8) -> u8 {}

            mod inner {
                #[kaalang]
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
            render_source("#[kaalang] fn empty() {}", "empty"),
            Err(RenderError::InvalidFlow { .. })
        ));
        assert_eq!(
            render_source(
                "#[kaalang] fn duplicate() {} #[kaalang] fn duplicate() {}",
                "duplicate"
            ),
            Err(RenderError::AmbiguousFlow("duplicate".into()))
        );
    }

    #[test]
    fn reports_kaalang_arguments_as_an_invalid_flow() {
        let source = "#[kaalang(unexpected)]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Return the input\")]\n    let end = |input| { input };\n}\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidFlow {
                name: "invalid".into(),
                line: 1,
                column: 1,
                message: "#[kaalang] does not accept arguments".into(),
            })
        );
    }

    #[test]
    fn reports_model_errors_at_their_source_position() {
        let source = "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Copy the input\")]\n    let input = |input| { input };\n}\n";

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
        let source = r#"#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value")]
    let (yes, no) = |condition| { condition };

    #[action("Build the first value")]
    let selected = |yes| { 1 };

    #[action("Use the selected value")]
    let end = |selected| { selected };

    #[action("Build the later alternative")]
    let selected = |no| { 2 };
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
        let source = "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[action(\"bad\\0label\")]\n    let end = |input| { input };\n}\n";

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
        let source = "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[choice(\"Pick\")]\n    #[case(\"first\")]\n    #[case(\"bad\\0case\")]\n    let (a, b) = |input| {\n        match input { 0 => (), _ => () }\n    };\n\n    #[action(\"A\")]\n    let end = |a| { 1 };\n\n    #[action(\"B\")]\n    let end = |b| { 2 };\n}\n";

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

    #[test]
    fn reports_an_invalid_question_branch_description() {
        let source = "#[kaalang]\nfn invalid(condition: bool) -> u8 {\n    #[question(\"Choose\")]\n    #[yes(\"bad\\0branch\")]\n    #[no]\n    let (yes, no) = |condition| { condition };\n    #[action(\"Yes\")]\n    let end = |yes| { 1 };\n    #[action(\"No\")]\n    let end = |no| { 0 };\n}\n";

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
}
