//! Reads flows and types from the text of a Rust source file, for a renderer
//! to draw: what one compilation target compiles of the file, the flow or type
//! a caller selects in it, and the text a diagram quotes from the source as
//! authored.
//!
//! Positions come from `proc-macro2`'s `span-locations`, which the `source`
//! feature enables. The macro leaves it off: rustc gives it tokens, not text.

use std::{cell::OnceCell, collections::BTreeMap, error::Error, fmt, ops::Range};

use proc_macro2::{Ident, Span};
use syn::{
    Attribute, Fields, Generics, ImplItem, Item, Meta, ReturnType, Signature, TraitItem,
    Visibility, spanned::Spanned,
};

pub use self::{
    cards::NamedPath,
    modules::{ExternalModule, ModuleDirectory, ModuleFile},
    ribbons::Ribbons,
};
use crate::{
    FlowItem, Owner, SemanticModel, Target, TypeDeclaration, TypeForm, build_with_options,
    datatype::parts, flows, type_declaration, type_items,
};

mod cards;
mod modules;
mod ribbons;
mod spans;

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
    /// How many bytes of the given text precede the tokenized text.
    prefix: usize,
    /// What the target compiles of the file.
    items: Vec<Item>,
    /// Where each function and type the file declares starts, at its first
    /// attribute, a `cfg` the target removed included, by where its name
    /// starts.
    starts: BTreeMap<usize, usize>,
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
        let mut starts = BTreeMap::new();
        collect_starts(&file.items, &mut starts);
        target.configure(&mut file);
        let prefix = source_prefix(text, &file);
        Ok(Self {
            tokenized: &text[prefix..],
            prefix,
            items: file.items,
            starts,
            types: OnceCell::new(),
        })
    }

    /// The flows and types the file declares that kaalang draws, in source
    /// order: a flow by its position and path as [`SourceFile::flow_named`]
    /// takes them, a type by its path as [`SourceFile::ty`] takes it. Each runs
    /// from its first attribute, a `cfg` that holds included, to just past its
    /// last character, where an editor's caret still stands in it, in bytes of
    /// the text as given. Where two meet, the one starting there takes the
    /// caret. A byte order mark, a shebang line, the space between
    /// declarations, and what the target does not compile declare nothing.
    #[must_use]
    pub fn declared_ranges(&self) -> Vec<(Range<usize>, Declared)> {
        let start = |name: &Ident, otherwise| {
            self.prefix
                + self
                    .starts
                    .get(&name.span().byte_range().start)
                    .copied()
                    .unwrap_or(otherwise)
        };
        let mut ranges = Vec::new();
        for (position, flow) in flows(&self.items).iter().enumerate() {
            let function = &flow.function;
            let end = self.prefix + function.block.brace_token.span.close().byte_range().end;
            ranges.push((
                start(
                    &function.sig.ident,
                    function.sig.fn_token.span.byte_range().start,
                )..end,
                Declared::Flow(position, flow_path(flow)),
            ));
        }
        for (path, item) in self.type_items() {
            if let (Some((_, name, _)), Some(bounds)) = (parts(item), type_bounds(item)) {
                ranges.push((
                    start(name, bounds.start)..self.prefix + bounds.end,
                    Declared::Type(path.clone()),
                ));
            }
        }
        ranges.sort_unstable_by_key(|(range, _)| range.start);
        ranges
    }

    /// What the target compiles of the file, for a caller to list its flows
    /// with [`flows`] or its types with [`type_items`].
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The bytes under `span`, a span of what was read from this file,
    /// counted from the start of the text as given: a byte order mark and a
    /// shebang line count.
    #[must_use]
    pub fn range(&self, span: Span) -> Range<usize> {
        let range = span.byte_range();
        self.prefix + range.start..self.prefix + range.end
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
        let item = flows(&self.items)
            .into_iter()
            .nth(position)
            .ok_or(SourceError::FlowNotFound(position))?;
        self.read_flow(item, collapse_cycles)
    }

    /// Reads the flow at the file-local `path` from [`Declared::Flow`], even
    /// after edits have moved it from `position`. If the path occurs more
    /// than once, its previous position takes precedence. Returns `None`
    /// when the file no longer declares that flow.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::InvalidFlow`] when the selected function is
    /// not a valid kaalang flow.
    pub fn flow_named(
        &self,
        path: &str,
        position: usize,
        collapse_cycles: bool,
    ) -> Result<Option<SourceFlow>, SourceError> {
        let mut flows = flows(&self.items);
        let matches = |flow: &FlowItem| flow_path(flow) == path;
        let item = if flows.get(position).is_some_and(matches) {
            Some(flows.swap_remove(position))
        } else {
            flows.into_iter().find(matches)
        };
        item.map(|item| self.read_flow(item, collapse_cycles))
            .transpose()
    }

    fn read_flow(&self, item: FlowItem, collapse_cycles: bool) -> Result<SourceFlow, SourceError> {
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
        // Only code rustc rejects declares a path twice for one target.
        let Some((_, item)) = self.type_items().iter().find(|(listed, _)| listed == path) else {
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
        let types = field_types(&declaration.form);
        let field_types = types.iter().map(|ty| self.collapsed(ty.span())).collect();
        let field_paths = types.iter().map(|ty| self.named_paths(ty)).collect();
        let relationship_targets: Vec<String> = declaration
            .relationships()
            .iter()
            .map(|relationship| self.collapsed(relationship.target.span()))
            .collect();
        // Every form `type_items` lists has a name.
        let Some((_, name, _)) = parts(item) else {
            return Err(SourceError::TypeNotFound(path.to_owned()));
        };
        let name = name.clone();
        let target_spans = declaration
            .relationships()
            .iter()
            .zip(&relationship_targets)
            .map(|(relationship, target)| {
                // `Self` names the declaring type.
                if target == "Self" {
                    return name.span();
                }
                cards::target_span(&relationship.target)
            })
            .collect();
        Ok(SourceType {
            path: path.to_owned(),
            name,
            generics: self.generics(&declaration.generics),
            representations: self.representations(item),
            field_types,
            field_paths,
            relationship_targets,
            target_spans,
            declaration,
            item: item.clone(),
        })
    }

    /// Every type declaration with its path within the file.
    fn type_items(&self) -> &[(String, Item)] {
        self.types.get_or_init(|| type_items(&self.items))
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

/// Bytes syn strips before tokenizing a whole Rust file.
pub(crate) fn source_prefix(text: &str, file: &syn::File) -> usize {
    usize::from(text.starts_with('\u{feff}')) * '\u{feff}'.len_utf8()
        + file.shebang.as_ref().map_or(0, String::len)
}

/// What a place in a source file declares that kaalang draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Declared {
    /// A flow, by its position among the file's flows and its file-local path.
    Flow(usize, String),
    /// A type, by its path within the file.
    Type(String),
}

/// A method's path names its owner within its module; a free function's
/// already includes its inline modules.
fn flow_path(flow: &FlowItem) -> String {
    match flow.owner {
        Owner::Module => flow.path.clone(),
        Owner::Impl(_) | Owner::Trait(_) => format!("{}{}", flow.module, flow.path),
    }
}

/// Records where each function and type among `items` that has attributes
/// starts, at its first one, by where its name starts, before the target
/// removes any of them. One without attributes starts where its span does.
fn collect_starts(items: &[Item], starts: &mut BTreeMap<usize, usize>) {
    for item in items {
        let (attributes, name) = match item {
            Item::Fn(function) => (&function.attrs, &function.sig.ident),
            Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_starts(items, starts);
                }
                continue;
            }
            Item::Impl(block) => {
                for item in &block.items {
                    if let ImplItem::Fn(function) = item {
                        record(&function.attrs, &function.sig.ident, starts);
                    }
                }
                continue;
            }
            Item::Trait(declaration) => {
                for item in &declaration.items {
                    if let TraitItem::Fn(function) = item {
                        record(&function.attrs, &function.sig.ident, starts);
                    }
                }
                continue;
            }
            item => match parts(item) {
                Some((attributes, name, _)) => (attributes, name),
                None => continue,
            },
        };
        record(attributes, name, starts);
    }
}

/// Where a type item runs without its attributes, from its first token to its
/// last: a whole item's span would rebuild its tokens.
fn type_bounds(item: &Item) -> Option<Range<usize>> {
    let (visibility, keyword, end) = match item {
        Item::Struct(declaration) => (
            &declaration.vis,
            declaration.struct_token.span,
            match (&declaration.fields, declaration.semi_token) {
                (_, Some(semi)) => semi.spans[0],
                (Fields::Named(fields), None) => fields.brace_token.span.close(),
                _ => return None,
            },
        ),
        Item::Enum(declaration) => (
            &declaration.vis,
            declaration.enum_token.span,
            declaration.brace_token.span.close(),
        ),
        Item::Union(declaration) => (
            &declaration.vis,
            declaration.union_token.span,
            declaration.fields.brace_token.span.close(),
        ),
        Item::Type(declaration) => (
            &declaration.vis,
            declaration.type_token.span,
            declaration.semi_token.spans[0],
        ),
        _ => return None,
    };
    let start = match visibility {
        Visibility::Public(token) => token.span,
        Visibility::Restricted(restricted) => restricted.pub_token.span,
        Visibility::Inherited => keyword,
    };
    Some(start.byte_range().start..end.byte_range().end)
}

/// Records where a declaration named `name` starts, at its first attribute.
fn record(attributes: &[Attribute], name: &Ident, starts: &mut BTreeMap<usize, usize>) {
    if let Some(first) = attributes.first() {
        starts.insert(
            name.span().byte_range().start,
            first.pound_token.span.byte_range().start,
        );
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
    /// Its name in its declaration.
    pub name: Ident,
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
    /// The paths each of `field_types` names.
    pub field_paths: Vec<Vec<NamedPath>>,
    /// The authored target of every relationship, in the order
    /// [`TypeDeclaration::relationships`] lists them.
    pub relationship_targets: Vec<String>,
    /// Where each of `relationship_targets` names its type, for an engine to
    /// resolve: its last segment, or the declaring type's name for `Self`.
    pub target_spans: Vec<Span>,
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

/// The type of every field of `form` in source order, a variant's fields in its
/// variant's place, or an alias's right-hand side alone.
fn field_types(form: &TypeForm) -> Vec<&syn::Type> {
    match form {
        TypeForm::Struct(fields) | TypeForm::Union(fields) => {
            fields.iter().map(|field| &field.ty).collect()
        }
        TypeForm::Enum(variants) => variants
            .iter()
            .flat_map(|variant| &variant.fields)
            .map(|field| &field.ty)
            .collect(),
        TypeForm::Alias(ty, _) => vec![ty.as_ref()],
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

    /// What `source`, read for `target`, declares where `marker` starts.
    fn declared(source: &str, target: &Target, marker: &str) -> Option<Declared> {
        let offset = source.find(marker).expect("the source has the marker");
        declared_at(&SourceFile::parse(source, target).unwrap(), offset)
    }

    /// What `file` declares at `offset`, as an editor finds the caret's
    /// declaration among the ranges: the last one that holds it.
    fn declared_at(file: &SourceFile<'_>, offset: usize) -> Option<Declared> {
        file.declared_ranges()
            .into_iter()
            .rfind(|(range, _)| range.contains(&offset) || range.end == offset)
            .map(|(_, declared)| declared)
    }

    const DECLARED: &str = "/// A flow.\n#[kaa]\nfn first(input: u8) -> u8 {\n    |input| return input;\n}\n\nmod model {\n    pub struct Member { id: u8 }\n    impl Member {\n        #[kaa]\n        fn second(input: u8) -> u8 {\n            |input| return input;\n        }\n    }\n}\n";

    #[test]
    fn a_flow_is_declared_from_its_first_attribute_to_just_past_its_end() {
        for marker in ["/// A flow", "#[kaa]\nfn first", "first(", "\n\nmod"] {
            assert_eq!(
                declared(DECLARED, Target::EMPTY, marker),
                Some(Declared::Flow(0, "first".into())),
                "{marker:?}"
            );
        }
    }

    #[test]
    fn a_method_in_a_module_is_declared_as_a_flow() {
        assert_eq!(
            declared(DECLARED, Target::EMPTY, "fn second"),
            Some(Declared::Flow(1, "model::Member::second".into()))
        );
    }

    #[test]
    fn a_type_in_a_module_is_declared_by_its_path() {
        assert_eq!(
            declared(DECLARED, Target::EMPTY, "id: u8"),
            Some(Declared::Type("model::Member".into()))
        );
    }

    #[test]
    fn nothing_is_declared_between_declarations_or_in_a_module_header() {
        for marker in ["\nmod", "mod model", "impl Member"] {
            assert_eq!(
                declared(DECLARED, Target::EMPTY, marker),
                None,
                "{marker:?}"
            );
        }
    }

    #[test]
    fn a_cfg_that_holds_starts_its_declaration() {
        let source = "#[cfg(unix)]\n#[kaa]\nfn flow(input: u8) -> u8 {\n    |input| return input;\n}\n#[cfg(unix)]\nstruct Kept;\n#[cfg(windows)]\nstruct Removed;\n";
        let unix: Target = "unix".parse().unwrap();

        assert_eq!(
            declared(source, &unix, "#[cfg(unix)]"),
            Some(Declared::Flow(0, "flow".into()))
        );
        assert_eq!(
            declared(source, &unix, "#[cfg(unix)]\nstruct"),
            Some(Declared::Type("Kept".into()))
        );
        assert_eq!(declared(source, &unix, "struct Removed"), None);
    }

    #[test]
    fn a_flow_marked_through_a_cfg_attr_is_declared_from_it() {
        let source =
            "#[cfg_attr(unix, kaa)]\nfn flow(input: u8) -> u8 {\n    |input| return input;\n}\n";
        let unix: Target = "unix".parse().unwrap();

        assert_eq!(
            declared(source, &unix, "#[cfg_attr"),
            Some(Declared::Flow(0, "flow".into()))
        );
        assert_eq!(declared(source, Target::EMPTY, "fn flow"), None);
    }

    #[test]
    fn offsets_count_the_byte_order_mark_and_the_shebang() {
        let source = format!("\u{feff}#!/usr/bin/env rustx\n{DECLARED}");

        assert_eq!(declared(&source, Target::EMPTY, "\u{feff}"), None);
        assert_eq!(declared(&source, Target::EMPTY, "rustx"), None);
        assert_eq!(
            declared(&source, Target::EMPTY, "first("),
            Some(Declared::Flow(0, "first".into()))
        );
    }

    #[test]
    fn text_that_is_not_rust_is_a_parse_error() {
        assert!(matches!(flow_error("fn"), SourceError::Parse { .. }));
    }

    #[test]
    fn the_next_declaration_owns_the_byte_where_it_starts() {
        let source = "#[kaa] fn a() {}#[kaa] fn b() {}struct C;struct D;#[kaa] fn e() {}";
        for (marker, expected) in [
            ("#[kaa] fn b", Declared::Flow(1, "b".into())),
            ("struct C", Declared::Type("C".into())),
            ("struct D", Declared::Type("D".into())),
            ("#[kaa] fn e", Declared::Flow(2, "e".into())),
        ] {
            assert_eq!(declared(source, Target::EMPTY, marker), Some(expected));
        }
    }

    #[test]
    fn a_moved_method_is_found_by_its_path_including_its_module() {
        let source = "mod a { impl Owner { #[kaa] fn run() { || return; } } }\nmod b { impl Owner { #[kaa] fn run() { || return; } } }";
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();
        let Declared::Flow(position, path) =
            declared_at(&file, source.rfind("run").unwrap()).unwrap()
        else {
            panic!("the cursor is in a flow");
        };
        let edited = format!("#[kaa] fn start() {{ || return; }}\n{source}");
        let file = SourceFile::parse(&edited, Target::EMPTY).unwrap();
        let method = file.flow_named(&path, position, false).unwrap().unwrap();
        assert_eq!(method.item.module, "b::");
        assert!(
            file.flow_named("b::Owner::gone", position, false)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn the_position_disambiguates_a_flow_path_while_editing() {
        let source = "#[kaa] fn run() { || return; }\n#[kaa] fn run(input: u8) -> u8 { |input| return input; }";
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();
        let flow = file.flow_named("run", 1, false).unwrap().unwrap();
        assert_eq!(flow.parameters, ["input: u8"]);
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
    fn a_flow_takes_one_active_attribute() {
        let source = format!("#[kaa]\n#[cfg_attr(unix, kaalang::kaa)]\n{COPY}");
        let enabled = SourceFile::parse(&source, &"unix".parse().unwrap()).unwrap();
        assert!(matches!(
            enabled.flow(0, false),
            Err(SourceError::InvalidFlow { line: 2, message, .. })
                if message == "a function takes one `#[kaa]` attribute"
        ));
        let disabled = SourceFile::parse(&source, Target::EMPTY).unwrap();
        assert!(disabled.flow(0, false).is_ok());
    }

    #[test]
    fn a_foreign_attribute_and_raw_cfg_keep_their_own_meanings() {
        let source = r#"
            mod sample {
                use other::{helper as kaa};
                #[kaalang::kaa] #[kaa("a foreign option")]
                fn flow(#[r#cfg_attr(all(), r#cfg(any()))] (a,b): (u8,u8), value: u8) -> u8 {
                    |value| return value;
                }
                struct Destination;
                #[kaalang::kaa("The owner.")] #[kaa("a foreign option")]
                struct Owner<#[r#cfg(any())] Destination> {
                    #[r#cfg_attr(all(), kaa::refer(Destination, "The destination."))] id: u8,
                }
            }
        "#;
        let source = SourceFile::parse(source, Target::EMPTY).unwrap();
        assert_eq!(source.flow(0, false).unwrap().parameters, ["value: u8"]);
        let owner = source.ty("sample::Owner").unwrap();
        assert_eq!(owner.declaration.description.as_deref(), Some("The owner."));
        assert_eq!(owner.declaration.relationships().len(), 1);
    }

    #[test]
    fn a_parent_import_preserves_the_identity_of_bare_kaa() {
        for local in ["super::super::kaa", "crate::kaa"] {
            for (root, duplicate) in [("kaalang::kaa", true), ("other::kaa", false)] {
                let source = format!(
                    "use {root}; mod a {{ mod b {{ use {local}; #[kaa] #[kaalang::kaa] {COPY} }} }}"
                );
                let file = SourceFile::parse(&source, Target::EMPTY).unwrap();
                let flow = file.flow(0, false);
                if duplicate {
                    assert!(matches!(flow, Err(SourceError::InvalidFlow { message, .. })
                        if message == "a function takes one `#[kaa]` attribute"));
                } else {
                    assert!(flow.is_ok());
                }
            }
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
