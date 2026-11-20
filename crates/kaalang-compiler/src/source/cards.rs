//! Where the text of a type's card stands in its source, which of a file's
//! relationships an engine's references to a type reach, and the order of the
//! cards' Cargo targets: facts a renderer and an editor's page report, with no
//! say in what a reader does with them.

use std::{collections::BTreeSet, ops::Range};

use proc_macro2::Span;
use syn::{
    Item, Path, PathSegment, QSelf, Type, UseTree, ext::IdentExt, spanned::Spanned, visit::Visit,
};

use super::SourceFile;
use crate::{TypeDeclaration, type_declaration};

/// One path a type's text names, as a card quotes the text.
#[derive(Clone, Debug)]
pub struct NamedPath {
    /// Where the path stands in the collapsed text, without generic arguments.
    pub range: Range<usize>,
    /// Its last segment, where an engine resolves the path.
    pub span: Span,
}

/// The Cargo target containing a type card, in RFC 0009 §4.3 order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CardCrate<'a> {
    pub package: &'a str,
    /// Library and binaries, tests, examples, then benches.
    pub group: u8,
    pub name: &'a str,
    pub root: &'a std::path::Path,
}

impl<'a> CardCrate<'a> {
    /// The focus's crate comes first, then the other targets of its package.
    #[must_use]
    pub fn order(self, focus_package: Option<&str>, is_focus: bool) -> (bool, bool, Self) {
        (!is_focus, Some(self.package) != focus_package, self)
    }

    /// Each package's target groups start separate rows in the diagram.
    #[must_use]
    pub fn group_key(self) -> (&'a str, u8) {
        (self.package, self.group)
    }
}

impl SourceFile<'_> {
    /// Every path `ty` names, in the order it writes them: an outer path before
    /// those in its generic arguments, and in `<T as Trait>::Item` the paths of
    /// `T`, then `Trait`, then `Item`.
    pub(super) fn named_paths(&self, ty: &Type) -> Vec<NamedPath> {
        #[derive(Default)]
        struct Paths {
            ranges: Vec<(Range<usize>, Span)>,
            trait_segments: Option<usize>,
        }
        impl Paths {
            /// Generic arguments break a path's visible text in two:
            /// `Type<Arg>::Member` names all three separately.
            fn segments<'a>(
                &mut self,
                mut start: Option<Span>,
                mut segments: impl Iterator<Item = &'a PathSegment>,
            ) {
                while let Some(mut last) = segments.next() {
                    let start = start.take().unwrap_or_else(|| last.ident.span());
                    while last.arguments.is_none() {
                        let Some(next) = segments.next() else { break };
                        last = next;
                    }
                    let end = last.ident.span();
                    self.ranges
                        .push((start.byte_range().start..end.byte_range().end, end));
                    self.visit_path_arguments(&last.arguments);
                }
            }
        }
        impl<'ast> Visit<'ast> for Paths {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                let named = self.trait_segments.take().unwrap_or(path.segments.len());
                let mut segments = path.segments.iter();
                self.segments(
                    path.leading_colon.map(|colon| colon.spans[0]),
                    segments.by_ref().take(named),
                );
                self.segments(None, segments);
            }

            fn visit_qself(&mut self, qself: &'ast QSelf) {
                // syn visits a qualified self immediately before its path,
                // in expressions and patterns as well as types.
                self.visit_type(&qself.ty);
                self.trait_segments = Some(qself.position);
            }
        }
        let mut paths = Paths::default();
        paths.visit_type(ty);
        paths.ranges.sort_unstable_by_key(|(range, _)| range.start);
        // Count each whitespace run once as the card's quoted text does.
        let mut byte = ty.span().byte_range().start;
        let mut collapsed = 0;
        let mut space = true;
        let mut advance = |end| {
            for ch in self.tokenized[byte..end].chars() {
                if ch.is_whitespace() {
                    collapsed += usize::from(!space);
                    space = true;
                } else {
                    collapsed += ch.len_utf8();
                    space = false;
                }
            }
            byte = end;
            collapsed
        };
        paths
            .ranges
            .into_iter()
            .map(|(range, span)| NamedPath {
                range: advance(range.start)..advance(range.end),
                span,
            })
            .collect()
    }

    /// The types of the file whose relationships target what stands at
    /// `offsets`, bytes of the text as given, where an engine finds the
    /// references to one type: each by its path within the file, with the
    /// positions of those relationships, in source order. An offset on a `use`
    /// that renames the type also reaches relationships naming that import by
    /// a relative path within the file, which an engine leaves out.
    #[must_use]
    pub fn referring(&self, offsets: &[usize]) -> Vec<(String, Vec<usize>)> {
        if offsets.is_empty() {
            return Vec::new();
        }
        let offsets: BTreeSet<_> = offsets.iter().copied().collect();
        let hit = |span: Span| offsets.range(self.range(span)).next().is_some();
        let mut renamed = BTreeSet::new();
        renames(&self.items, "", &hit, &mut renamed);
        self.declarations()
            .filter_map(|(path, declaration)| {
                let module = path.rsplit_once("::").map_or("", |(module, _)| module);
                let positions: Vec<usize> = (0..)
                    .zip(declaration.relationships())
                    .filter(|(_, relationship)| {
                        // An engine finds a reference to a type at its last
                        // segment; one before it names a module or the type of
                        // a variant.
                        let target = &relationship.target;
                        hit(target_span(target)) || names_import(target, module, &renamed)
                    })
                    .map(|(position, _)| position)
                    .collect();
                (!positions.is_empty()).then(|| (path.to_owned(), positions))
            })
            .collect()
    }

    /// The byte where each relationship's target names its last segment in
    /// the given text, so an editor can resolve that name to its definition.
    #[must_use]
    pub fn relationship_offsets(&self) -> Vec<usize> {
        let mut offsets = Vec::new();
        for (_, declaration) in self.declarations() {
            for relationship in declaration.relationships() {
                offsets.push(self.range(target_span(&relationship.target)).start);
            }
        }
        offsets
    }

    /// The declarations an incoming card can draw, as [`SourceFile::ty`]
    /// selects them when the current text repeats a type's path.
    fn declarations(&self) -> impl Iterator<Item = (&str, TypeDeclaration)> {
        let mut seen = BTreeSet::new();
        self.type_items().iter().filter_map(move |(path, item)| {
            if !seen.insert(path.as_str()) {
                return None;
            }
            type_declaration(item)
                .ok()
                .map(|declaration| (path.as_str(), declaration))
        })
    }
}

/// The final segment of an authored relationship path, including `Self`.
pub(super) fn target_span(target: &Path) -> Span {
    target
        .segments
        .last()
        .expect("a relationship names a target")
        .ident
        .span()
}

/// Whether a relative `path` from `module` leads to a renamed import. A
/// file alone cannot locate the crate root or follow a path out of the file.
fn names_import(path: &Path, module: &str, renamed: &BTreeSet<String>) -> bool {
    if renamed.is_empty() || path.leading_colon.is_some() {
        return false;
    }
    let mut parts: Vec<String> = module
        .split("::")
        .filter(|part| !part.is_empty())
        .map(|part| part.trim_start_matches("r#").to_owned())
        .collect();
    for segment in &path.segments {
        match segment.ident.to_string().as_str() {
            "self" => {}
            "super" => {
                if parts.pop().is_none() {
                    return false;
                }
            }
            "crate" => return false,
            name => parts.push(name.trim_start_matches("r#").to_owned()),
        }
    }
    renamed.contains(&parts.join("::"))
}

/// Collects the file-local path of every name a `use` among `items` in
/// `module`, or in a module inside it, gives a path whose renamed segment `hit`s.
fn renames(
    items: &[Item],
    module: &str,
    hit: &impl Fn(Span) -> bool,
    renamed: &mut BTreeSet<String>,
) {
    for item in items {
        match item {
            Item::Use(declaration) => renamed_in(&declaration.tree, module, hit, renamed),
            Item::Mod(inner) => {
                if let Some((_, items)) = &inner.content {
                    let path = format!("{module}{}::", inner.ident.unraw());
                    renames(items, &path, hit, renamed);
                }
            }
            _ => {}
        }
    }
}

/// [`renames`] within one `use` tree.
fn renamed_in(
    tree: &UseTree,
    module: &str,
    hit: &impl Fn(Span) -> bool,
    renamed: &mut BTreeSet<String>,
) {
    match tree {
        UseTree::Path(path) => renamed_in(&path.tree, module, hit, renamed),
        UseTree::Group(group) => {
            for tree in &group.items {
                renamed_in(tree, module, hit, renamed);
            }
        }
        UseTree::Rename(rename) if hit(rename.ident.span()) => {
            renamed.insert(format!("{module}{}", rename.rename.unraw()));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::{SourceFile, Target};

    /// The text each name of each field's type covers, and the source its
    /// span covers.
    fn names(source: &str, path: &str) -> Vec<Vec<(String, String)>> {
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();
        let ty = file.ty(path).unwrap();
        ty.field_paths
            .iter()
            .zip(&ty.field_types)
            .map(|(names, text)| {
                names
                    .iter()
                    .map(|name| {
                        (
                            text[name.range.clone()].to_owned(),
                            source[file.range(name.span)].to_owned(),
                        )
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_field_type_names_each_path_it_writes() {
        let source = "struct Report {\n    list: std::vec::Vec<  model::Item >,\n    pair: (u8, ::core::cell::Cell<i8>),\n}\n";
        assert_eq!(
            names(source, "Report"),
            [
                vec![
                    ("std::vec::Vec".into(), "Vec".into()),
                    ("model::Item".into(), "Item".into()),
                ],
                vec![
                    ("u8".into(), "u8".into()),
                    ("::core::cell::Cell".into(), "Cell".into()),
                    ("i8".into(), "i8".into()),
                ],
            ]
        );
    }

    #[test]
    fn a_comment_or_a_line_break_in_a_type_keeps_its_names_in_place() {
        let source = "struct Report {\n    list: Vec<\n        /* the items */ Item,\n    >,\n}\n";
        assert_eq!(
            names(source, "Report"),
            [vec![
                ("Vec".into(), "Vec".into()),
                ("Item".into(), "Item".into()),
            ]]
        );
    }

    #[test]
    fn a_qualified_path_names_its_type_its_trait_and_what_follows_apart() {
        assert_eq!(
            names(
                "struct Items {\n    it: <Vec<Member> as IntoIterator>::Item,\n}\n",
                "Items"
            ),
            [vec![
                ("Vec".into(), "Vec".into()),
                ("Member".into(), "Member".into()),
                ("IntoIterator".into(), "IntoIterator".into()),
                ("Item".into(), "Item".into()),
            ]]
        );
    }

    #[test]
    fn paths_in_an_array_length_keep_generic_arguments_and_members_apart() {
        assert_eq!(
            names(
                "type Bytes = [u8; Size::<Item>::N + <Item as ::limits::Bound<Size>>::N];",
                "Bytes"
            ),
            [vec![
                ("u8".into(), "u8".into()),
                ("Size".into(), "Size".into()),
                ("Item".into(), "Item".into()),
                ("N".into(), "N".into()),
                ("Item".into(), "Item".into()),
                ("::limits::Bound".into(), "Bound".into()),
                ("Size".into(), "Size".into()),
                ("N".into(), "N".into()),
            ]]
        );
    }

    #[test]
    fn an_alias_names_the_paths_of_its_right_hand_side() {
        assert_eq!(
            names("type Members = Vec<Member>;", "Members"),
            [vec![
                ("Vec".into(), "Vec".into()),
                ("Member".into(), "Member".into()),
            ]]
        );
    }

    const REFERRING: &str = r#"use kaalang::kaa;

use crate::model::Member as Person;

#[kaa]
pub struct Review {
    #[kaa::refer(Person, "Through a renamed import.")]
    author: u8,
    #[kaa::refer(crate::model::Member, "Through a path.")]
    editor: u8,
    #[kaa::refer(Other, "Something else.")]
    other: u8,
}

pub fn uses(member: crate::model::Member) {}

mod inner {
    use kaalang::kaa;

    #[kaa]
    pub struct Elsewhere {
        #[kaa::refer(Person, "Another module's name.")]
        author: u8,
    }
}
"#;

    /// The types `REFERRING` refers from to what stands at each `marker`.
    fn referring(markers: &[&str]) -> Vec<(String, Vec<usize>)> {
        let file = SourceFile::parse(REFERRING, Target::EMPTY).unwrap();
        let offsets: Vec<usize> = markers
            .iter()
            .map(|marker| REFERRING.find(marker).unwrap())
            .collect();
        file.referring(&offsets)
    }

    #[test]
    fn a_reference_in_a_relationship_target_reaches_that_relationship() {
        assert_eq!(
            referring(&["Member, \"Through a path"]),
            [("Review".to_owned(), vec![1])]
        );
    }

    #[test]
    fn a_reference_before_a_targets_last_segment_reaches_nothing() {
        let source = "#[kaa]\nenum Member { Lead }\n\n#[kaa]\nstruct Team {\n    #[kaa::refer(Member::Lead, \"A variant.\")]\n    lead: u8,\n}\n";
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();
        assert_eq!(file.referring(&[source.find("Member::").unwrap()]), []);
    }

    #[test]
    fn a_path_declared_twice_refers_as_its_first_declaration() {
        let source = "#[kaa]\nstruct Team {\n    #[kaa::refer(Member, \"One.\")]\n    lead: u8,\n}\n\n#[kaa]\nstruct Team {\n    #[kaa::refer(Other, \"Two.\")]\n    other: u8,\n    #[kaa::refer(Member, \"Three.\")]\n    lead: u8,\n}\n";
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();
        let offsets: Vec<usize> = source.match_indices("Member").map(|(at, _)| at).collect();
        assert_eq!(file.referring(&offsets), [("Team".to_owned(), vec![0])]);
    }

    #[test]
    fn a_reference_elsewhere_reaches_nothing() {
        assert_eq!(referring(&["Member) {}"]), []);
    }

    #[test]
    fn a_definition_is_requested_at_each_target_the_file_can_draw() {
        let source = format!(
            "\u{feff}#!/usr/bin/env rustx\n{}",
            r#"
            #[kaa]
            struct Page {
                #[cfg_attr(unix, kaa::refer(crate::model::Member, "The author."))]
                author: u8,
                #[cfg(windows)] #[kaa::refer(Other, "Other.")] other: u8,
                #[kaa::refer(Self, "The previous page.")] previous: u8,
            }
            #[kaa] struct Broken {
                #[kaa::refer(Bad, "First.")] #[kaa::refer(Bad, "Second.")] bad: u8,
            }
            #[kaa] #[kaa::refer(Page, "The page.")] type Owned = u8;
        "#
        );
        let file = SourceFile::parse(&source, &"unix".parse().unwrap()).unwrap();
        let offsets = file.relationship_offsets();
        assert_eq!(
            offsets,
            [
                source.find("Member,").unwrap(),
                source.find("Self,").unwrap(),
                source.find("Page,").unwrap(),
            ]
        );
        assert_eq!(
            file.referring(&offsets),
            [("Page".into(), vec![0, 1]), ("Owned".into(), vec![0]),]
        );
    }

    #[test]
    fn a_renamed_import_reaches_the_relationships_of_its_module_that_use_the_name() {
        assert_eq!(
            referring(&["Member as Person"]),
            [("Review".to_owned(), vec![0])]
        );
    }

    #[test]
    fn a_relative_path_can_reach_a_renamed_import_across_inline_modules() {
        let source = r#"mod model { pub struct Member; }
        mod r#labels {
            pub(crate) use crate::model::Member as r#Person;
            #[kaa] struct Team {
                #[kaa::refer(self::Person, "Same module.")] lead: u8,
            }
            mod inner {
                #[kaa] struct Review {
                    #[kaa::refer(super::r#Person, "Parent module.")] author: u8,
                }
            }
        }
        #[kaa] struct Group {
            #[kaa::refer(labels::Person, "Child module.")] lead: u8,
        }"#;
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();
        let name = source.find("Member as").unwrap();
        assert_eq!(
            file.referring(&[name]),
            [
                ("r#labels::Team".into(), vec![0]),
                ("r#labels::inner::Review".into(), vec![0]),
                ("Group".into(), vec![0]),
            ]
        );
    }

    #[test]
    fn a_self_target_names_its_owner() {
        let source = "#[kaa]\nstruct Member {\n    #[kaa::refer(Self, \"The previous one.\")]\n    previous: u8,\n}\n";
        let file = SourceFile::parse(source, Target::EMPTY).unwrap();
        let ty = file.ty("Member").unwrap();
        assert_eq!(ty.relationship_targets, ["Self"]);
        assert_eq!(&source[file.range(ty.target_spans[0])], "Member");
    }
}
