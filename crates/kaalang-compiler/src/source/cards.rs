//! Where the text of a type's card stands in its source, and which of a file's
//! relationships an engine's references to a type reach: facts a renderer and
//! an editor's page report, with no say in what a reader does with them.

use std::{collections::BTreeSet, ops::Range};

use proc_macro2::Span;
use syn::{Item, PathSegment, Type, TypePath, UseTree, spanned::Spanned, visit::Visit};

use super::{SourceFile, collapsed_text};
use crate::type_declaration;

/// One path a type's text names, as a card quotes the text.
#[derive(Clone, Debug)]
pub struct NamedPath {
    /// Where the path stands in the collapsed text, without generic arguments.
    pub range: Range<usize>,
    /// Its last segment, where an engine resolves the path.
    pub span: Span,
}

impl SourceFile<'_> {
    /// Every path `ty` names, in the order it writes them: an outer path before
    /// those in its generic arguments, and in `<T as Trait>::Item` the paths of
    /// `T`, then `Trait`, then `Item`.
    pub(super) fn named_paths(&self, ty: &Type) -> Vec<NamedPath> {
        struct Paths(Vec<(Range<usize>, Span)>);
        impl Paths {
            /// Records the path of `segments`, from `start` when given.
            fn push<'a>(
                &mut self,
                start: Option<Span>,
                segments: impl IntoIterator<Item = &'a PathSegment>,
            ) {
                let segments: Vec<&PathSegment> = segments.into_iter().collect();
                if let (Some(first), Some(last)) = (segments.first(), segments.last()) {
                    let start = start.unwrap_or_else(|| first.ident.span());
                    let end = last.ident.span();
                    self.0
                        .push((start.byte_range().start..end.byte_range().end, end));
                }
            }
        }
        impl<'ast> Visit<'ast> for Paths {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                self.push(
                    path.leading_colon.map(|colon| colon.spans[0]),
                    &path.segments,
                );
                syn::visit::visit_path(self, path);
            }

            fn visit_type_path(&mut self, ty: &'ast TypePath) {
                let Some(qself) = &ty.qself else {
                    syn::visit::visit_type_path(self, ty);
                    return;
                };
                // A qualified path writes its trait and what follows apart.
                self.visit_type(&qself.ty);
                let segments: Vec<&PathSegment> = ty.path.segments.iter().collect();
                let (named, rest) = segments.split_at(qself.position);
                self.push(None, named.iter().copied());
                self.push(None, rest.iter().copied());
                for segment in &ty.path.segments {
                    self.visit_path_arguments(&segment.arguments);
                }
            }
        }
        let mut paths = Paths(Vec::new());
        paths.visit_type(ty);
        let whole = ty.span().byte_range();
        let text = &self.tokenized[whole.clone()];
        paths
            .0
            .into_iter()
            .map(|(range, span)| NamedPath {
                range: collapsed_offset(text, range.start - whole.start)
                    ..collapsed_offset(text, range.end - whole.start),
                span,
            })
            .collect()
    }

    /// The types of the file whose relationships target what stands at
    /// `offsets`, bytes of the text as given, where an engine finds the
    /// references to one type: each by its path within the file, with the
    /// positions of those relationships, in source order. An offset on a `use`
    /// that renames the type also reaches the relationships of its module that
    /// name it by the new name, which an engine leaves out. An offset anywhere
    /// else reaches nothing.
    #[must_use]
    pub fn referring(&self, offsets: &[usize]) -> Vec<(String, Vec<usize>)> {
        let hit = |span: Span| {
            let range = self.range(span);
            offsets.iter().any(|offset| range.contains(offset))
        };
        let mut renamed = BTreeSet::new();
        renames(&self.items, "", &hit, &mut renamed);
        let mut seen = BTreeSet::new();
        self.type_items()
            .iter()
            // Only code rustc rejects declares a path twice; `ty` reads the first.
            .filter(|(path, _)| seen.insert(path.as_str()))
            .filter_map(|(path, item)| {
                let declaration = type_declaration(item).ok()?;
                let module = path.rsplit_once("::").map_or("", |(module, _)| module);
                let positions: Vec<usize> = (0..)
                    .zip(declaration.relationships())
                    .filter(|(_, relationship)| {
                        // An engine finds a reference to a type at its last
                        // segment; one before it names a module or the type of
                        // a variant.
                        let target = &relationship.target;
                        target
                            .segments
                            .last()
                            .is_some_and(|last| hit(last.ident.span()))
                            || target.get_ident().is_some_and(|name| {
                                renamed.contains(&(module.to_owned(), name.to_string()))
                            })
                    })
                    .map(|(position, _)| position)
                    .collect();
                (!positions.is_empty()).then(|| (path.clone(), positions))
            })
            .collect()
    }
}

/// Collects, as module path and name, every name a `use` among `items` in
/// `module`, or in a module inside it, gives a path whose renamed segment
/// `hit`s.
fn renames(
    items: &[Item],
    module: &str,
    hit: &impl Fn(Span) -> bool,
    renamed: &mut BTreeSet<(String, String)>,
) {
    for item in items {
        match item {
            Item::Use(declaration) => renamed_in(&declaration.tree, module, hit, renamed),
            Item::Mod(inner) => {
                if let Some((_, items)) = &inner.content {
                    let path = if module.is_empty() {
                        inner.ident.to_string()
                    } else {
                        format!("{module}::{}", inner.ident)
                    };
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
    renamed: &mut BTreeSet<(String, String)>,
) {
    match tree {
        UseTree::Path(path) => renamed_in(&path.tree, module, hit, renamed),
        UseTree::Group(group) => {
            for tree in &group.items {
                renamed_in(tree, module, hit, renamed);
            }
        }
        UseTree::Rename(rename) if hit(rename.ident.span()) => {
            renamed.insert((module.to_owned(), rename.rename.to_string()));
        }
        _ => {}
    }
}

/// Where byte `at` of `text`, the start or the end of a token, falls in
/// `collapsed_text(text)`: a whitespace run before it counts as one space,
/// unless nothing precedes it.
fn collapsed_offset(text: &str, at: usize) -> usize {
    let before = &text[..at];
    let collapsed = collapsed_text(before).len();
    if collapsed > 0 && before.ends_with(char::is_whitespace) {
        collapsed + 1
    } else {
        collapsed
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
    fn a_renamed_import_reaches_the_relationships_of_its_module_that_use_the_name() {
        assert_eq!(
            referring(&["Member as Person"]),
            [("Review".to_owned(), vec![0])]
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
