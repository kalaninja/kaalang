//! Resolves a `refer` target to the declaration it names, by Rust's path rules
//! over the workspace's module trees: `crate`, `self`, and `super`, declared
//! items, `use` imports with renames, groups, and globs, and the workspace's
//! libraries by their crate names. rustc has already checked that the path
//! names something; this finds what.

use syn::ext::IdentExt;

use crate::workspace::{Crate, Declared, UsePath};

/// A type declaration: its crate and its index among the crate's types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TypeRef {
    pub(super) krate: usize,
    pub(super) id: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Resolution {
    /// A struct, enum, union, or alias of the workspace.
    Type(TypeRef),
    /// Something of the workspace that is not a type, such as "a trait".
    NotAType(&'static str),
    /// A name from outside the workspace: a primitive, the standard library,
    /// or a dependency.
    Outside,
    /// The source alone cannot tell, for the reason given.
    Unknown(&'static str),
}

/// What a path resolves to on the way to its last segment.
enum Def {
    Type(TypeRef),
    Module(usize, usize),
    NotAType(&'static str),
    Outside,
    Unknown(&'static str),
}

pub(super) struct Resolver<'a> {
    crates: &'a [Crate],
    /// Each crate's names for the workspace's libraries.
    externs: Vec<Vec<(String, usize)>>,
}

impl<'a> Resolver<'a> {
    pub(super) fn new(crates: &'a [Crate]) -> Self {
        let libraries: Vec<(usize, &Crate)> = crates
            .iter()
            .enumerate()
            .filter(|(_, krate)| krate.library)
            .collect();
        let externs = crates
            .iter()
            .map(|krate| {
                let mut names: Vec<(String, usize)> = libraries
                    .iter()
                    .map(|(index, library)| (library.name.clone(), *index))
                    .collect();
                for (rename, package) in &krate.renames {
                    if let Some((index, _)) = libraries
                        .iter()
                        .find(|(_, library)| library.package == *package)
                    {
                        names.push((rename.clone(), *index));
                    }
                }
                names
            })
            .collect();
        Self { crates, externs }
    }

    /// Resolves `path` as written in `module` of crate `krate`.
    pub(super) fn resolve(&self, krate: usize, module: usize, path: &syn::Path) -> Resolution {
        let path = UsePath {
            global: path.leading_colon.is_some(),
            segments: path
                .segments
                .iter()
                .map(|segment| segment.ident.unraw().to_string())
                .collect(),
        };
        match self.path(krate, module, &path, &mut Vec::new()) {
            Def::Type(found) => Resolution::Type(found),
            Def::Module(..) => Resolution::NotAType("a module"),
            Def::NotAType(what) => Resolution::NotAType(what),
            Def::Outside => Resolution::Outside,
            Def::Unknown(reason) => Resolution::Unknown(reason),
        }
    }

    /// Resolves a path as written in `module`: its first segment from the
    /// module's scope or the crates it may name, the rest through modules.
    fn path(&self, krate: usize, module: usize, path: &UsePath, visiting: &mut Visiting) -> Def {
        let Some((first, rest)) = path.segments.split_first() else {
            return Def::Unknown("the path is empty");
        };
        let mut current = if path.global {
            self.extern_crate(krate, first)
        } else {
            match first.as_str() {
                "crate" => Def::Module(krate, 0),
                "self" => Def::Module(krate, module),
                "super" => self.parent(krate, module),
                _ => self.first(krate, module, first, visiting),
            }
        };
        for segment in rest {
            current = match current {
                Def::Module(krate, module) => match segment.as_str() {
                    "super" => self.parent(krate, module),
                    "self" => Def::Module(krate, module),
                    _ => self
                        .member(krate, module, segment, visiting)
                        // rustc found it, so the walk missed it.
                        .unwrap_or(Def::Unknown("its module does not declare it as read")),
                },
                // An enum variant or an associated item.
                Def::Type(_) | Def::NotAType(_) => Def::NotAType("an associated item"),
                outside_or_unknown => return outside_or_unknown,
            };
        }
        current
    }

    /// A relative path's first segment: a name in the module's scope, a
    /// workspace library, or a name from outside the workspace.
    fn first(&self, krate: usize, module: usize, name: &str, visiting: &mut Visiting) -> Def {
        match self.member(krate, module, name, visiting) {
            // A glob the walk could not follow may declare it, but a workspace
            // library of that name is what the path most likely means.
            Some(Def::Unknown(reason)) => self.library(krate, name).unwrap_or(Def::Unknown(reason)),
            Some(found) => found,
            None => self.library(krate, name).unwrap_or(Def::Outside),
        }
    }

    /// The crate root `name` stands for in a `::` path or `extern crate`.
    fn extern_crate(&self, krate: usize, name: &str) -> Def {
        if name == "self" || name == "crate" {
            return Def::Module(krate, 0);
        }
        self.library(krate, name).unwrap_or(Def::Outside)
    }

    /// The root of the workspace library that `krate` names `name`.
    fn library(&self, krate: usize, name: &str) -> Option<Def> {
        self.externs[krate]
            .iter()
            .find(|(extern_name, _)| extern_name == name)
            .map(|(_, library)| Def::Module(*library, 0))
    }

    fn parent(&self, krate: usize, module: usize) -> Def {
        self.crates[krate].modules[module]
            .parent
            .map_or(Def::Unknown("`super` leaves the crate"), |parent| {
                Def::Module(krate, parent)
            })
    }

    /// What `name` stands for in the type namespace of `module`, then among
    /// its values: declared there, imported by name, or through a glob.
    // shortcut: visibility is not checked, so a glob may import a private name rustc would not; upgrade by reading each item's visibility.
    fn member(
        &self,
        krate: usize,
        module: usize,
        name: &str,
        visiting: &mut Visiting,
    ) -> Option<Def> {
        // A `use` cycle, such as two globs importing each other, ends here.
        let key = (krate, module, name.to_owned());
        if visiting.contains(&key) {
            return None;
        }
        visiting.push(key);
        let found = self.member_unguarded(krate, module, name, visiting);
        visiting.pop();
        found
    }

    fn member_unguarded(
        &self,
        krate: usize,
        module: usize,
        name: &str,
        visiting: &mut Visiting,
    ) -> Option<Def> {
        let scope = &self.crates[krate].modules[module].scope;
        if let Some((_, declared)) = scope.declared.iter().find(|(declared, _)| declared == name) {
            return Some(match declared {
                Declared::Type(id) => Def::Type(TypeRef { krate, id: *id }),
                Declared::Module(child) => Def::Module(krate, *child),
                Declared::Trait => Def::NotAType("a trait"),
                Declared::ExternCrate(named) => self.extern_crate(krate, named),
            });
        }
        if let Some((_, imported)) = scope.imports.iter().find(|(bound, _)| bound == name) {
            return Some(self.path(krate, module, imported, visiting));
        }
        if let Some((_, what)) = scope.values.iter().find(|(value, _)| value == name) {
            return Some(Def::NotAType(what));
        }
        let mut unknown = None;
        for glob in &scope.globs {
            match self.path(krate, module, glob, visiting) {
                Def::Module(source, inner) => {
                    if let Some(found) = self.member(source, inner, name, visiting) {
                        return Some(found);
                    }
                }
                Def::Unknown(reason) => unknown = Some(Def::Unknown(reason)),
                // A name a dependency's glob imports is outside the workspace,
                // as a name found nowhere is; a glob over an enum's variants
                // imports no type.
                Def::Outside | Def::Type(_) | Def::NotAType(_) => {}
            }
        }
        // shortcut: a name an item macro declares is not seen, so a module with one cannot tell it from the prelude; upgrade by expanding the macro.
        if unknown.is_none() && scope.opaque {
            unknown = Some(Def::Unknown(
                "its module may declare it through an item macro or a file that was not read",
            ));
        }
        unknown
    }
}

type Visiting = Vec<(usize, usize, String)>;

#[cfg(test)]
mod tests {
    use super::{Resolution, Resolver, TypeRef};
    use crate::workspace::{Crate, library};

    /// Resolves `target` from the module at `module`, a path such as `a::b::`,
    /// in the first of `crates`.
    fn resolve(crates: &[Crate], module: &str, target: &str) -> Resolution {
        let module = crates[0]
            .modules
            .iter()
            .position(|candidate| candidate.path == module)
            .expect("the module exists");
        let target: syn::Path = syn::parse_str(target).expect("the target parses");
        Resolver::new(crates).resolve(0, module, &target)
    }

    /// The type of crate `krate` at `path` from its root.
    fn named(crates: &[Crate], krate: usize, path: &str) -> Resolution {
        let id = crates[krate]
            .types
            .iter()
            .position(|entry| entry.path == path)
            .expect("the type exists");
        Resolution::Type(TypeRef { krate, id })
    }

    #[test]
    fn declared_types_resolve_through_crate_self_and_super() {
        let crates = [library(
            "app",
            "struct Member; mod a { struct Team; mod b { struct Leader; } }",
        )];
        for (module, target, name) in [
            ("", "Member", "Member"),
            ("", "a::Team", "a::Team"),
            ("a::", "self::b::Leader", "a::b::Leader"),
            ("a::b::", "super::Team", "a::Team"),
            ("a::b::", "super::super::Member", "Member"),
            ("a::b::", "crate::a::b::Leader", "a::b::Leader"),
            ("", "r#a::r#Team", "a::Team"),
        ] {
            assert_eq!(
                resolve(&crates, module, target),
                named(&crates, 0, name),
                "{target}"
            );
        }
    }

    #[test]
    fn imports_resolve_with_renames_groups_and_globs() {
        let crates = [library(
            "app",
            "mod model { pub struct Member; pub struct Team; pub mod inner { pub struct Deep; } }
             mod named { use crate::model::Member; }
             mod renamed { use crate::model::Member as Person; }
             mod grouped { use crate::model::{Team, inner::{self, Deep as Nested}}; }
             mod globbed { use crate::model::*; }",
        )];
        for (module, target, name) in [
            ("named::", "Member", "model::Member"),
            ("renamed::", "Person", "model::Member"),
            ("grouped::", "Team", "model::Team"),
            ("grouped::", "inner::Deep", "model::inner::Deep"),
            ("grouped::", "Nested", "model::inner::Deep"),
            ("globbed::", "Team", "model::Team"),
            ("globbed::", "inner::Deep", "model::inner::Deep"),
        ] {
            assert_eq!(
                resolve(&crates, module, target),
                named(&crates, 0, name),
                "{target}"
            );
        }
    }

    #[test]
    fn re_exports_resolve_through_chains_and_glob_cycles_end() {
        let crates = [library(
            "app",
            "mod model { pub struct Member; }
             mod first { pub use crate::model::Member as Person; }
             mod second { pub use crate::first::*; }
             mod left { pub use crate::right::*; }
             mod right { pub use crate::left::*; }",
        )];
        assert_eq!(
            resolve(&crates, "", "second::Person"),
            named(&crates, 0, "model::Member")
        );
        assert!(matches!(
            resolve(&crates, "", "left::Missing"),
            Resolution::Unknown(_)
        ));
    }

    #[test]
    fn workspace_libraries_resolve_by_crate_name() {
        let crates = [
            library("app", ""),
            library("model-types", "pub mod people { pub struct Member; }"),
        ];
        assert_eq!(
            resolve(&crates, "", "model_types::people::Member"),
            named(&crates, 1, "people::Member")
        );
        assert_eq!(
            resolve(&crates, "", "::model_types::people::Member"),
            named(&crates, 1, "people::Member")
        );
    }

    #[test]
    fn globs_do_not_hide_a_workspace_library() {
        let crates = [
            // One glob leaves the workspace and one cannot be followed.
            library("app", "use std::collections::*;\nuse self::generated::*;"),
            library("model-types", "pub struct Member;"),
        ];
        assert_eq!(
            resolve(&crates, "", "model_types::Member"),
            named(&crates, 1, "Member")
        );
    }

    #[test]
    fn a_renamed_dependency_resolves_to_its_library() {
        let mut app = library("app", "");
        app.renames
            .push(("people".to_owned(), "model-types".to_owned()));
        let crates = [app, library("model-types", "pub struct Member;")];
        assert_eq!(
            resolve(&crates, "", "people::Member"),
            named(&crates, 1, "Member")
        );
    }

    #[test]
    fn items_that_are_not_types_are_named() {
        let crates = [library(
            "app",
            "trait Named {} mod model {} fn build() {} const LIMIT: u8 = 1; enum State { Open }",
        )];
        for (target, what) in [
            ("Named", "a trait"),
            ("model", "a module"),
            ("build", "a function"),
            ("LIMIT", "a constant"),
            ("State::Open", "an associated item"),
        ] {
            assert_eq!(
                resolve(&crates, "", target),
                Resolution::NotAType(what),
                "{target}"
            );
        }
    }

    #[test]
    fn names_from_outside_the_workspace_are_outside() {
        let crates = [library("app", "use std::collections::HashMap;")];
        for target in [
            "String",
            "u32",
            "std::string::String",
            "HashMap",
            "serde::Value",
        ] {
            assert_eq!(
                resolve(&crates, "", target),
                Resolution::Outside,
                "{target}"
            );
        }
    }

    #[test]
    fn a_glob_into_a_module_with_an_item_macro_leaves_an_unknown_name_unknown() {
        let crates = [library(
            "app",
            "mod generated { declare_types!(Member); } use self::generated::*;",
        )];
        assert!(matches!(
            resolve(&crates, "", "Member"),
            Resolution::Unknown(_)
        ));
    }

    #[test]
    fn an_item_macro_leaves_an_unknown_name_unknown() {
        let crates = [library("app", "declare_types!(Member);")];
        assert!(matches!(
            resolve(&crates, "", "Member"),
            Resolution::Unknown(_)
        ));
    }
}
