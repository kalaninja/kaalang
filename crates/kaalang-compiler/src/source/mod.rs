//! Reads flows and types from the text of a Rust source file, for a renderer
//! to draw: what one compilation target compiles of the file, the flow or type
//! a caller selects in it, and the text a diagram quotes from the source as
//! authored.
//!
//! Positions come from `proc-macro2`'s `span-locations`, which the `source`
//! feature enables. The macro leaves it off: rustc gives it tokens, not text.

use std::{cell::OnceCell, error::Error, fmt};

use proc_macro2::Span;
use syn::{Generics, Item, Meta, ReturnType, Signature, spanned::Spanned};

pub use self::target::Target;
use crate::{
    FlowItem, SemanticModel, TypeDeclaration, TypeForm, build_with_options, flows,
    is_kaalang_attribute, type_declaration, type_items,
};

mod target;

/// An error produced while reading a flow or a type from a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// The input is not a syntactically valid Rust file.
    Parse {
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// No `#[kaa]` function stands at the requested position among the file's
    /// flows.
    FlowNotFound(usize),
    /// The selected function is not a valid kaalang flow.
    InvalidFlow {
        name: String,
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
    /// No type has the requested path.
    TypeNotFound(String),
    /// The type has annotations that are not valid.
    InvalidType {
        name: String,
        /// The 1-based source line and column where the error starts.
        line: usize,
        column: usize,
        message: String,
    },
}

impl fmt::Display for SourceError {
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
            Self::FlowNotFound(position) => {
                write!(
                    formatter,
                    "no `#[kaa]` function stands at position {position}"
                )
            }
            Self::InvalidFlow {
                name,
                line,
                column,
                message,
            } => write!(
                formatter,
                "invalid kaalang flow `{name}` at {line}:{column}: {message}"
            ),
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
        }
    }
}

impl Error for SourceError {}

/// A Rust source file as one compilation target compiles it.
pub struct SourceFile<'a> {
    /// The text the parser tokenized, which positions count from: the given
    /// text without a byte order mark or a shebang line. The newline that ends
    /// a shebang line stays, so line numbers match the given text.
    tokenized: &'a str,
    /// What the target compiles of the file.
    items: Vec<Item>,
    /// Every type declaration with its path within the file, listed on first
    /// use: a flow's diagram needs none.
    types: OnceCell<Vec<(String, Item)>>,
}

impl<'a> SourceFile<'a> {
    /// Parses `text` and removes what `target` does not compile.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::Parse`] when `text` is not valid Rust.
    pub fn parse(text: &'a str, target: &Target) -> Result<Self, SourceError> {
        let mut file = syn::parse_file(text).map_err(|error| {
            let (line, column) = line_and_column(error.span());
            SourceError::Parse {
                line,
                column,
                message: error.to_string(),
            }
        })?;
        target.configure(&mut file);
        let tokenized = text.strip_prefix('\u{feff}').unwrap_or(text);
        let tokenized = file
            .shebang
            .as_ref()
            .map_or(tokenized, |shebang| &tokenized[shebang.len()..]);
        Ok(Self {
            tokenized,
            items: file.items,
            types: OnceCell::new(),
        })
    }

    /// What the target compiles of the file, for a caller to list its flows
    /// with [`flows`] or its types with [`type_items`].
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Reads the flow at `position` among the file's flows, counted in source
    /// order as [`flows`] lists them, with each cycle as one node when
    /// `collapse_cycles` is set. The caller selected it by its own rules.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::FlowNotFound`] when no flow stands at `position`
    /// and [`SourceError::InvalidFlow`] when that function is not a valid
    /// kaalang flow.
    pub fn flow(&self, position: usize, collapse_cycles: bool) -> Result<SourceFlow, SourceError> {
        let Some(item) = flows(&self.items).into_iter().nth(position) else {
            return Err(SourceError::FlowNotFound(position));
        };
        if let Some(attribute) = item
            .function
            .attrs
            .iter()
            .filter(|attribute| is_kaalang_attribute(attribute))
            .find(|attribute| match &attribute.meta {
                Meta::Path(_) => false,
                Meta::List(arguments) => !arguments.tokens.is_empty(),
                Meta::NameValue(_) => true,
            })
        {
            return Err(invalid_flow(
                &item.path,
                &syn::Error::new_spanned(
                    attribute,
                    "#[kaa] on a function does not accept arguments",
                ),
            ));
        }
        let model = build_with_options(&item.function, collapse_cycles)
            .map_err(|error| invalid_flow(&item.path, &error))?;
        let signature = &item.function.sig;
        Ok(SourceFlow {
            header: self.header(signature),
            parameters: signature
                .inputs
                .iter()
                .map(|parameter| self.collapsed(parameter.span()))
                .collect(),
            return_type: match &signature.output {
                ReturnType::Default => "()".to_owned(),
                ReturnType::Type(_, ty) => self.collapsed(ty.span()),
            },
            model,
            item,
        })
    }

    /// Reads the type at `path` within the file, as [`type_items`] lists it.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::TypeNotFound`] when `path` names no type the
    /// target compiles and [`SourceError::InvalidType`] when that type's
    /// annotations are not valid.
    pub fn ty(&self, path: &str) -> Result<SourceType, SourceError> {
        let types = self.types.get_or_init(|| type_items(&self.items));
        // Only code rustc rejects declares a path twice for one target.
        let Some((_, item)) = types.iter().find(|(listed, _)| listed == path) else {
            return Err(SourceError::TypeNotFound(path.to_owned()));
        };
        let declaration = type_declaration(item).map_err(|error| {
            let (line, column) = line_and_column(error.span());
            SourceError::InvalidType {
                name: path.to_owned(),
                line,
                column,
                message: error.to_string(),
            }
        })?;
        let field_types = match &declaration.form {
            TypeForm::Struct(fields) | TypeForm::Union(fields) => fields
                .iter()
                .map(|field| self.collapsed(field.ty.span()))
                .collect(),
            TypeForm::Enum(variants) => variants
                .iter()
                .flat_map(|variant| &variant.fields)
                .map(|field| self.collapsed(field.ty.span()))
                .collect(),
            TypeForm::Alias(ty, _) => vec![self.collapsed(ty.span())],
        };
        let relationship_targets = declaration
            .relationships()
            .iter()
            .map(|relationship| self.collapsed(relationship.target.span()))
            .collect();
        Ok(SourceType {
            path: path.to_owned(),
            generics: self.generics(&declaration.generics),
            representations: self.representations(item),
            field_types,
            relationship_targets,
            declaration,
            item: item.clone(),
        })
    }

    /// The authored text under a span with every whitespace run collapsed to
    /// one space.
    fn collapsed(&self, span: Span) -> String {
        collapsed_text(&self.tokenized[span.byte_range()])
    }

    /// The generic parameters a declaration keeps for its target, each as
    /// authored, between angle brackets, or nothing when it keeps none.
    fn generics(&self, generics: &Generics) -> String {
        if generics.params.is_empty() {
            return String::new();
        }
        let parameters: Vec<String> = generics
            .params
            .iter()
            .map(|parameter| self.collapsed(parameter.span()))
            .collect();
        format!("<{}>", parameters.join(", "))
    }

    /// The authored flow header without its parameters and return type.
    fn header(&self, signature: &Signature) -> String {
        let start = signature.span().byte_range().start;
        // The generics follow from the parameters the target keeps.
        let end = signature.generics.lt_token.map_or_else(
            || signature.paren_token.span.open().byte_range().start,
            |open| open.span.byte_range().start,
        );
        let keyword = signature.fn_token.span.byte_range();
        let mut header = self.tokenized[start..end].to_owned();
        header.replace_range(keyword.start - start..keyword.end - start, "");
        let mut authored = collapsed_text(&header);
        authored.push_str(&self.generics(&signature.generics));
        if let Some(where_clause) = &signature.generics.where_clause {
            authored.push(' ');
            authored.push_str(&self.collapsed(where_clause.span()));
        }
        authored
    }

    /// The authored arguments of each `repr` attribute of a struct, an enum,
    /// or a union. A `repr` under a `cfg_attr` that holds is one of them,
    /// since configuring the file expanded it.
    fn representations(&self, item: &Item) -> Vec<String> {
        let attributes = match item {
            Item::Struct(declaration) => &declaration.attrs,
            Item::Enum(declaration) => &declaration.attrs,
            Item::Union(declaration) => &declaration.attrs,
            _ => return Vec::new(),
        };
        attributes
            .iter()
            .filter(|attribute| attribute.path().is_ident("repr"))
            .filter_map(|attribute| match &attribute.meta {
                Meta::List(list) => {
                    let delimiters = list.delimiter.span();
                    Some(collapsed_text(
                        &self.tokenized[delimiters.open().byte_range().end
                            ..delimiters.close().byte_range().start],
                    ))
                }
                _ => None,
            })
            // An empty `repr()` leaves the default, as Rust reads it.
            .filter(|representation| !representation.is_empty())
            .collect()
    }
}

/// One flow of a source file, validated, with the text its diagram quotes from
/// its signature. Only [`SourceFile::flow`] makes one.
#[non_exhaustive]
pub struct SourceFlow {
    /// The function, with its path within the file.
    pub item: FlowItem,
    /// Its model, with each cycle as one node when the caller asked for that.
    pub model: SemanticModel,
    /// The authored header without its parameters and return type: the
    /// qualifiers and the name, then the generic parameters the target keeps
    /// and the where clause.
    pub header: String,
    /// The authored parameters the target keeps, in order.
    pub parameters: Vec<String>,
    /// The authored return type, or `()` when the signature has none.
    pub return_type: String,
}

/// One type of a source file, with the text its card quotes from the
/// declaration. Only [`SourceFile::ty`] makes one.
#[non_exhaustive]
pub struct SourceType {
    /// The type's path within the file.
    pub path: String,
    /// The declaration as the target compiles it.
    pub item: Item,
    /// Its form, generics, description, and annotations.
    pub declaration: TypeDeclaration,
    /// The generic parameters the target keeps, each as authored, between
    /// angle brackets, or nothing when it keeps none.
    pub generics: String,
    /// The authored arguments of each `repr` attribute that has any, in source
    /// order. An alias has none.
    pub representations: Vec<String>,
    /// The authored type of every field in source order, a variant's fields in
    /// its variant's place, or an alias's right-hand side alone.
    pub field_types: Vec<String>,
    /// The authored target of every relationship, in the order
    /// [`TypeDeclaration::relationships`] lists them.
    pub relationship_targets: Vec<String>,
}

/// The 1-based line and column where `span` starts in the text its file was
/// parsed from.
#[must_use]
pub fn line_and_column(span: Span) -> (usize, usize) {
    let start = span.start();
    (start.line, start.column + 1)
}

/// Reports a flow-level error at its source position.
fn invalid_flow(name: &str, error: &syn::Error) -> SourceError {
    let (line, column) = line_and_column(error.span());
    SourceError::InvalidFlow {
        name: name.to_owned(),
        line,
        column,
        message: error.to_string(),
    }
}

fn collapsed_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const COPY: &str = "fn copy(input: u8) -> u8 {\n    #[kaa::action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";

    /// What reading the first flow of `source` for no target fails with.
    fn flow_error(source: &str) -> SourceError {
        SourceFile::parse(source, Target::EMPTY)
            .and_then(|file| file.flow(0, false).map(|_| ()))
            .expect_err("the source declares no valid flow")
    }

    #[test]
    fn a_type_is_selected_by_its_path_annotated_or_not() {
        let source = "mod model { #[kaa(\"\")] struct Empty; }\nstruct Plain { id: u32 }\n";
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();

        assert_eq!(
            file.ty("Missing").err(),
            Some(SourceError::TypeNotFound("Missing".into()))
        );
        assert!(matches!(
            file.ty("model::Empty"),
            Err(SourceError::InvalidType { name, line: 1, .. }) if name == "model::Empty"
        ));
        assert!(file.ty("Plain").is_ok_and(|ty| ty.field_types == ["u32"]));
    }

    #[test]
    fn a_type_is_read_as_its_target_compiles_it() {
        let source = "#[cfg(unix)]\nstruct Handle(u8, #[cfg(windows)] isize);\n#[cfg(not(unix))]\nstruct Handle(u16);\n#[cfg(windows)]\nstruct Raw;\n";
        let unix: Target = "unix".parse().unwrap();
        let types = |target| {
            let file = SourceFile::parse(source, target).unwrap();
            file.ty("Handle").map(|ty| ty.field_types).ok()
        };

        assert_eq!(types(&unix), Some(vec!["u8".to_owned()]));
        assert_eq!(types(Target::EMPTY), Some(vec!["u16".to_owned()]));
        assert_eq!(
            SourceFile::parse(source, &unix).unwrap().ty("Raw").err(),
            Some(SourceError::TypeNotFound("Raw".into()))
        );
    }

    #[test]
    fn a_type_lists_its_fields_and_relationship_targets_in_source_order() {
        let source = r#"#[kaa] enum Approval {
            Named(#[kaa::refer(Member, "One.")] Vec< u8 >),
            Quorum { votes: u16, #[kaa::refer(directory :: Team, "Two.")] voters: u32 },
        }
        #[kaa] #[kaa::refer(Member, "The member.")] type MemberRef = u64;"#;
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();

        let approval = file
            .ty("Approval")
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(approval.field_types, ["Vec< u8 >", "u16", "u32"]);
        assert_eq!(
            approval.relationship_targets,
            ["Member", "directory :: Team"]
        );
        let alias = file
            .ty("MemberRef")
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(alias.field_types, ["u64"]);
        assert_eq!(alias.relationship_targets, ["Member"]);
    }

    #[test]
    fn a_representation_under_a_cfg_attr_counts_when_it_holds() {
        let source = "#[kaa] #[repr(C)] #[repr(align(8))] struct Combined(u8);\n#[kaa] #[cfg_attr(unix, cfg_attr(target_pointer_width = \"64\", repr(C)))] struct Conditional(u8);\n";
        let unix: Target = "unix\ntarget_pointer_width=\"64\"".parse().unwrap();
        let representations = |path, target| {
            let file = SourceFile::parse(source, target).unwrap();
            file.ty(path).map(|ty| ty.representations).ok()
        };

        assert_eq!(
            representations("Combined", Target::EMPTY),
            Some(vec!["C".to_owned(), "align(8)".to_owned()])
        );
        assert_eq!(
            representations("Conditional", &unix),
            Some(vec!["C".to_owned()])
        );
        assert_eq!(representations("Conditional", Target::EMPTY), Some(vec![]));
    }

    #[test]
    fn a_type_lists_the_generic_parameters_its_target_compiles() {
        let source = "struct Handle<#[cfg(windows)] W, #[cfg(unix)] U>(u8);\n";
        let unix: Target = "unix".parse().unwrap();
        let generics = |target| {
            let file = SourceFile::parse(source, target).unwrap();
            file.ty("Handle").map(|ty| ty.generics).ok()
        };

        assert_eq!(generics(&unix).as_deref(), Some("<U>"));
        assert_eq!(generics(Target::EMPTY).as_deref(), Some(""));
    }

    #[test]
    fn a_flow_lists_the_parameters_its_target_compiles() {
        let source = "#[kaa]\nfn copy<#[cfg(windows)] W, #[cfg(unix)] U>(#[cfg(windows)] raw: W, #[cfg(unix)] input: U) -> U {\n    #[kaa::action(\"Return the input\")]\n    let end = |input| { input };\n    |end| return end;\n}\n";
        let unix: Target = "unix".parse().unwrap();

        let flow = SourceFile::parse(source, &unix)
            .unwrap()
            .flow(0, false)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(flow.header, "copy<U>");
        assert_eq!(flow.parameters, ["input: U"]);
        assert_eq!(flow.return_type, "U");
    }

    #[test]
    fn a_header_keeps_its_qualifiers_and_where_clause_without_the_keyword() {
        let source = format!(
            "#[kaa]\npub const  unsafe fn copy<T>(input: u8) -> u8\nwhere\n    T: Copy,\n{}",
            COPY.split_once("-> u8 ").unwrap().1
        );

        let flow = SourceFile::parse(&source, Target::EMPTY)
            .unwrap()
            .flow(0, false)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(flow.header, "const unsafe copy<T> where T: Copy,");
    }

    #[test]
    fn a_flow_requires_an_exact_kaalang_attribute() {
        assert_eq!(
            flow_error("#[other::kaa]\nfn nested_attribute(input: u8) -> u8 {}\n"),
            SourceError::FlowNotFound(0)
        );
    }

    #[test]
    fn text_that_is_not_rust_is_a_parse_error() {
        assert!(matches!(flow_error("fn"), SourceError::Parse { .. }));
    }

    #[test]
    fn a_flow_marked_through_the_crate_path_is_found() {
        let source = format!("#[kaalang::kaa]\n{COPY}");

        assert!(
            SourceFile::parse(&source, Target::EMPTY)
                .unwrap()
                .flow(0, false)
                .is_ok()
        );
    }

    #[test]
    fn kaa_arguments_are_an_invalid_flow() {
        for attribute in ["kaa", "kaalang::kaa"] {
            assert_eq!(
                flow_error(&format!("#[{attribute}(unexpected)]\n{COPY}")),
                SourceError::InvalidFlow {
                    name: "copy".into(),
                    line: 1,
                    column: 1,
                    message: "#[kaa] on a function does not accept arguments".into(),
                }
            );
        }
    }

    #[test]
    fn a_model_error_is_reported_at_its_source_position() {
        let source = "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"Copy the input\")]\n    let input = |input| { input };\n    |input| return input;\n}\n";

        assert_eq!(
            flow_error(source),
            SourceError::InvalidFlow {
                name: "invalid".into(),
                line: 4,
                column: 9,
                message: "a kaalang block output must not reuse a flow input name".into(),
            }
        );
    }

    /// Language validation rejects this before a renderer can receive a plan
    /// that repeats the shared consumer.
    #[test]
    fn a_late_alternative_producer_is_an_invalid_flow() {
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
            flow_error(source),
            SourceError::InvalidFlow {
                name: "invalid".into(),
                line: 13,
                column: 9,
                message: "every producer of a kaalang wire must be declared before its consumers"
                    .into(),
            }
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

        let labels = |source: &str| {
            let flow = SourceFile::parse(source, Target::EMPTY)
                .unwrap()
                .flow(0, false)
                .unwrap_or_else(|error| panic!("{error}"));
            (flow.header, flow.parameters, flow.return_type)
        };
        let reference = labels(SOURCE);
        assert_eq!(
            reference,
            (
                "idéntity".to_owned(),
                vec!["value: u32".to_owned()],
                "u32".to_owned()
            )
        );
        for (prefix, _) in PREFIXES {
            assert_eq!(labels(&format!("{prefix}{SOURCE}")), reference);
        }
        assert_eq!(
            labels(&format!("#![allow(dead_code)]\n{SOURCE}")),
            reference
        );

        let invalid = [
            "fn invalid(",
            "#[kaa]\nfn invalid(input: u8) -> u8 {\n    #[kaa::action(\"Copy\")]\n    let input = |input| input;\n    |input| return input;\n}\n",
        ];
        let location = |error: &SourceError| match error {
            SourceError::Parse { line, column, .. }
            | SourceError::InvalidFlow { line, column, .. } => (*line, *column),
            other => panic!("expected a source-spanned error, got {other:?}"),
        };
        for source in invalid {
            let expected = flow_error(source);
            let (line, column) = location(&expected);
            for (prefix, added_lines) in PREFIXES {
                let error = flow_error(&format!("{prefix}{source}"));
                assert_eq!(location(&error), (line + added_lines, column));
                assert_eq!(
                    std::mem::discriminant(&error),
                    std::mem::discriminant(&expected)
                );
            }
        }
    }
}
