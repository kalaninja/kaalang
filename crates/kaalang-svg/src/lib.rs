//! Renders validated kaalang flows as standalone SVG diagrams.

use std::{error::Error, fmt, rc::Rc};

use proc_macro2::Span;
use syn::{File, Item, ItemFn, Meta, ReturnType, Signature, spanned::Spanned};

mod captions;
mod layout;
mod svg;
mod text;

/// Presentation options for one rendered flow.
#[derive(Clone, Copy, Default)]
pub struct RenderOptions {
    /// Replace every validated cycle region with one described loop node.
    pub collapse_loops: bool,
}

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
    /// No exact `#[kaalang]` function has the requested name.
    FlowNotFound(String),
    /// More than one exact `#[kaalang]` function has the requested name.
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
                write!(formatter, "`#[kaalang]` function `{name}` was not found")
            }
            Self::AmbiguousFlow(name) => write!(
                formatter,
                "multiple `#[kaalang]` functions are named `{name}`"
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

/// Renders one exact `#[kaalang]` function from a UTF-8 Rust source file.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust,
/// [`RenderError::FlowNotFound`] or [`RenderError::AmbiguousFlow`] when
/// `flow_name` does not name exactly one `#[kaalang]` function,
/// [`RenderError::InvalidFlow`] when that function is not a valid kaalang flow,
/// [`RenderError::InvalidLabelCharacter`] when a rendered label contains a
/// character XML 1.0 cannot represent, and
/// [`RenderError::UnroutableTopology`] when realizing the model's checked
/// arrangement as geometry cannot route every connection and place every label
/// under RFC 0002 §8.
pub fn render_source(source: &str, flow_name: &str) -> Result<String, RenderError> {
    render_source_with_options(source, flow_name, RenderOptions::default())
}

/// Renders one flow using the requested presentation options.
///
/// # Errors
///
/// Returns the same errors as [`render_source`].
pub fn render_source_with_options(
    source: &str,
    flow_name: &str,
    options: RenderOptions,
) -> Result<String, RenderError> {
    let file = parse_file(source)?;
    let parser_source = parser_source(source, &file);
    let function = select_flow(&file.items, flow_name)?;
    let mut model = kaalang_compiler::build_with_options(&function, options.collapse_loops)
        .map_err(|error| invalid_flow(flow_name, &error))?;
    kaalang_render::compact_arrangement(&mut model);
    let start = layout::start_text(parser_source, &function.sig);
    let parameters = layout::parameter_text(parser_source, &function.sig);
    let return_type = layout::return_text(parser_source, &function.sig.output);
    validate_labels(&model, &function.sig, &start, &parameters, &return_type)?;
    // Each formula is laid out once, here rather than per layout attempt.
    // Validation above has already parsed the same descriptions, without their
    // math, to read the characters a reader would see.
    let captions = Rc::new(captions::derive(&model, &start, &return_type));
    let scene = layout::layout(&model, &captions, &parameters).map_err(|reason| {
        RenderError::UnroutableTopology {
            name: flow_name.to_owned(),
            reason,
        }
    })?;

    Ok(svg::serialize(&scene, &model.analysis.name.to_string()))
}

/// Names every `#[kaalang]` function in a UTF-8 Rust source file, free or
/// associated, in source order.
///
/// # Errors
///
/// Returns [`RenderError::Parse`] when `source` is not valid Rust.
pub fn flow_names(source: &str) -> Result<Vec<String>, RenderError> {
    Ok(kaalang_compiler::flows(&parse_file(source)?.items)
        .iter()
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

/// The exact source suffix `syn::parse_file` tokenized. Its retained newline
/// after a shebang keeps span line numbers aligned with the original file.
fn parser_source<'a>(source: &'a str, file: &File) -> &'a str {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    file.shebang
        .as_ref()
        .map_or(source, |shebang| &source[shebang.len()..])
}

fn select_flow(items: &[Item], flow_name: &str) -> Result<ItemFn, RenderError> {
    let mut matches = kaalang_compiler::flows(items)
        .into_iter()
        .filter(|function| function.sig.ident == flow_name);
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

/// Checks authored descriptions and source-derived labels for invalid XML characters.
fn validate_labels(
    model: &kaalang_compiler::SemanticModel,
    signature: &Signature,
    start: &str,
    parameters: &[String],
    return_type: &str,
) -> Result<(), RenderError> {
    for block in &model.analysis.flow.blocks {
        if block.kind == kaalang_compiler::BlockKind::Call && block.description.is_none() {
            validate_label(&block.callee(), block.span, "call label")?;
        }
        let description = block
            .description
            .as_deref()
            .map(|text| (text, "block description".to_owned()));
        let cases = block
            .case_descriptions
            .iter()
            .enumerate()
            .map(|(case, text)| (text.as_str(), format!("case {} description", case + 1)));
        let branches = block
            .question_branches
            .iter()
            .enumerate()
            .filter_map(|(branch, answer)| {
                answer
                    .description
                    .as_deref()
                    .map(|text| (text, format!("question branch {} description", branch + 1)))
            });
        for (text, context) in description.into_iter().chain(cases).chain(branches) {
            validate_markdown_label(text, block.span, context)?;
        }
    }
    validate_label(start, signature.span(), "flow header")?;
    for (index, (parameter, text)) in signature.inputs.iter().zip(parameters).enumerate() {
        validate_label(text, parameter.span(), format!("parameter {}", index + 1))?;
    }
    if let ReturnType::Type(_, ty) = &signature.output {
        validate_label(return_type, ty.span(), "return type")?;
    }

    Ok(())
}

fn validate_markdown_label(
    label: &str,
    span: Span,
    context: impl Into<String>,
) -> Result<(), RenderError> {
    let context = context.into();
    validate_label(label, span, context.clone())?;
    let parsed = text::RichText::markdown_text(label);
    let Some(character) = parsed
        .spans()
        .iter()
        .flat_map(|span| span.text.chars())
        .find(|character| !valid_xml_character(*character))
    else {
        return Ok(());
    };
    let (line, column) = location(span);
    Err(RenderError::InvalidLabelCharacter {
        character,
        line,
        column,
        context,
    })
}

fn validate_label(label: &str, span: Span, context: impl Into<String>) -> Result<(), RenderError> {
    let Some(character) = invalid_xml_character(label) else {
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

/// Finds the first character XML 1.0 cannot represent.
fn invalid_xml_character(label: &str) -> Option<char> {
    label
        .chars()
        .find(|character| !valid_xml_character(*character))
}

const fn valid_xml_character(character: char) -> bool {
    matches!(
        character,
        '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
    )
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
        let source = "#[kaalang(unexpected)]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";

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
        let source = "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Copy the input\")]\n    let input = |input| { input };\n    |input| return input;\n}\n";

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
        let source = "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[action(\"bad\\0label\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";

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
        assert_eq!(invalid_xml_character("λ\t\n\r<&>"), None);

        for (source, line, column, context) in [
            (
                "#[kaalang]\nfn /*\0*/ invalid(input: u8) -> u8 {\n    |input| return input;\n}\n",
                2,
                1,
                "flow header",
            ),
            (
                "\u{feff}#!/usr/bin/env rustx\r\n#[kaalang]\nfn invalid(input: /*\0*/ u8) -> u8 {\n    |input| return input;\n}\n",
                3,
                12,
                "parameter 1",
            ),
            (
                "#[kaalang]\nfn invalid(input: u8) -> (u8, /*\0*/ u8) {\n    |input| return input;\n}\n",
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
        let source = "#[kaalang]\nfn invalid() -> usize {\n    #[call]\n    let output = count::<{r\"a\0b\".len()}>();\n    |output| return output;\n}\nfn count<const N: usize>() -> usize { N }\n";

        assert_eq!(
            render_source(source, "invalid"),
            Err(RenderError::InvalidLabelCharacter {
                character: '\0',
                line: 3,
                column: 5,
                context: "call label".into(),
            })
        );
        let described = source.replace("#[call]", "#[call(\"Count bytes.\")]");
        assert!(render_source(&described, "invalid").is_ok());
    }

    #[test]
    fn reports_an_invalid_case_description_by_its_position() {
        let source = "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[choice(\"Pick\")]\n    #[case(\"first\")]\n    #[case(\"bad\\0case\")]\n    let (a, b) = |input| {\n        match input { 0 => (), _ => () }\n    };\n\n    #[action(\"A\")]\n    let end = |a| { 1 };\n\n    #[action(\"B\")]\n    let end = |b| { 2 };\n\n    |end| return end;\n}\n";

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
    /// character it names is validated too (RFC 0005 §4).
    #[test]
    fn reports_an_invalid_character_a_reference_introduces() {
        let source = "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Record&#30;the run.\")]\n    let end = |input| { input };\n\n    |end| return end;\n}\n";

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
    fn reports_an_invalid_question_branch_description() {
        let source = "#[kaalang]\nfn invalid(condition: bool) -> u8 {\n    #[question(\"Choose\")]\n    #[yes(\"bad\\0branch\")]\n    #[no]\n    let (yes, no) = |condition| { condition };\n    #[action(\"Yes\")]\n    let end = |yes| { 1 };\n    #[action(\"No\")]\n    let end = |no| { 0 };\n    |end| return end;\n}\n";

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
            "#[kaalang]\nfn idéntity(value: u32) -> u32 {\n    |value| return value;\n}\n";
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
                "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[action(\"Copy\")]\n    let input = |input| input;\n    |input| return input;\n}\n",
                "invalid",
            ),
            (
                "#[kaalang]\nfn invalid(input: u8) -> u8 {\n    #[action(\"bad\\0label\")]\n    let output = |input| input;\n    |output| return output;\n}\n",
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
