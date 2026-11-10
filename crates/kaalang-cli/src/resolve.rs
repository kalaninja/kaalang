//! Resolves a `refer` target to the declaration it names, by Rust's path rules
//! over the workspace's module trees: `crate`, `self`, and `super`, declared
//! items, `use` imports with renames, groups, and globs, and the workspace's
//! libraries by their crate names. rustc has already checked that the path
//! names something; this finds what.

use std::iter;

use syn::ext::IdentExt;

use crate::workspace::{Crate, Declared, UsePath, Visibility};

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

/// What a path resolves to.
pub(super) enum Def {
    Type(TypeRef),
    Module(usize, usize),
    /// A trait: its crate, its module, and its name.
    Trait(usize, usize, String),
    /// A function, constant, or static: its crate, its module, its name, and
    /// what it is.
    Value(usize, usize, String, &'static str),
    NotAType(&'static str),
    Outside,
    /// A module the walk read in full does not declare the name.
    Missing,
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
                let mut names = Vec::new();
                for (name, package) in &krate.dependencies {
                    if let Some((index, _)) = libraries
                        .iter()
                        .find(|(_, library)| library.package_id == *package)
                    {
                        names.push((name.clone(), *index));
                    }
                }
                // A package's other targets can name its own library.
                if !krate.library {
                    names.extend(
                        libraries
                            .iter()
                            .filter(|(_, library)| library.package_id == krate.package_id)
                            .map(|(index, library)| (library.name.clone(), *index)),
                    );
                }
                names
            })
            .collect();
        Self { crates, externs }
    }

    /// Resolves `path` as written in `module` of crate `krate`.
    pub(super) fn resolve(&self, krate: usize, module: usize, path: &syn::Path) -> Resolution {
        match self.find(krate, module, path) {
            Def::Type(found) => Resolution::Type(found),
            Def::Module(..) => Resolution::NotAType("a module"),
            Def::Trait(..) => Resolution::NotAType("a trait"),
            Def::Value(.., what) | Def::NotAType(what) => Resolution::NotAType(what),
            Def::Outside => Resolution::Outside,
            // rustc found it, so the walk missed it.
            Def::Missing => Resolution::Unknown("its module does not declare it as read"),
            Def::Unknown(reason) => Resolution::Unknown(reason),
        }
    }

    /// What `path`, as written in `module` of crate `krate`, names.
    pub(super) fn find(&self, krate: usize, module: usize, path: &syn::Path) -> Def {
        self.lookup(krate, module, path, false)
    }

    /// What `path` names among functions, constants, and statics: its last
    /// segment passes over a module or type that shares the name, as in
    /// `mod checkout; pub use checkout::checkout;`.
    pub(super) fn find_value(&self, krate: usize, module: usize, path: &syn::Path) -> Def {
        self.lookup(krate, module, path, true)
    }

    fn lookup(&self, krate: usize, module: usize, path: &syn::Path, value: bool) -> Def {
        let path = UsePath {
            global: path.leading_colon.is_some(),
            segments: path
                .segments
                .iter()
                .map(|segment| segment.ident.unraw().to_string())
                .collect(),
        };
        self.path(krate, module, &path, value, &mut Vec::new())
    }

    /// Resolves a path as written in `module`: its first segment from the
    /// module's scope or the crates it may name, the rest through modules. A
    /// `value` path looks its last segment up among values.
    fn path(
        &self,
        krate: usize,
        module: usize,
        path: &UsePath,
        value: bool,
        visiting: &mut Visiting,
    ) -> Def {
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
                _ => self.first(krate, module, first, value && rest.is_empty(), visiting),
            }
        };
        for (index, segment) in rest.iter().enumerate() {
            let value = value && index + 1 == rest.len();
            current = match current {
                Def::Module(krate, module) => match segment.as_str() {
                    "super" => self.parent(krate, module),
                    "self" => Def::Module(krate, module),
                    _ => self
                        .member(krate, module, segment, value, visiting)
                        .map_or(Def::Missing, |(found, _)| found),
                },
                // An enum variant or an associated item.
                Def::Type(_) | Def::Trait(..) | Def::Value(..) | Def::NotAType(_) => {
                    Def::NotAType("an associated item")
                }
                outside_or_unknown => return outside_or_unknown,
            };
        }
        current
    }

    /// A relative path's first segment: a name in the module's scope, a
    /// workspace library, or a name from outside the workspace.
    fn first(
        &self,
        krate: usize,
        module: usize,
        name: &str,
        value: bool,
        visiting: &mut Visiting,
    ) -> Def {
        match self
            .member(krate, module, name, value, visiting)
            .map(|(found, _)| found)
        {
            // A glob the walk could not follow may declare it, but a workspace
            // library of that name is what the path most likely means.
            Some(unknown @ (Def::Unknown(_) | Def::Missing)) => {
                self.library(krate, name).unwrap_or(unknown)
            }
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
    /// its values, or only among its values when `value` is set: declared
    /// there, imported by name, or through a glob that sees it. Returns where
    /// in `krate` the name may be used from.
    fn member(
        &self,
        krate: usize,
        module: usize,
        name: &str,
        value: bool,
        visiting: &mut Visiting,
    ) -> Option<(Def, Visibility)> {
        let scope = &self.crates[krate].modules[module].scope;
        // A declared name is found even inside the `use` that imports a value
        // of that name, as `mod checkout; pub use checkout::checkout;` does.
        if !value
            && let Some((_, declared, visibility)) = scope
                .declared
                .iter()
                .find(|(declared, ..)| declared == name)
        {
            let found = match declared {
                Declared::Type(id) => Def::Type(TypeRef { krate, id: *id }),
                Declared::Module(child) => Def::Module(krate, *child),
                Declared::Trait => Def::Trait(krate, module, name.to_owned()),
                Declared::ExternCrate(named) => self.extern_crate(krate, named),
            };
            return Some((found, *visibility));
        }
        // A `use` cycle, such as two globs importing each other, ends here.
        let key = (krate, module, name.to_owned());
        if visiting.contains(&key) {
            return None;
        }
        visiting.push(key);
        let found = self.member_unguarded(krate, module, name, value, visiting);
        visiting.pop();
        found
    }

    fn member_unguarded(
        &self,
        krate: usize,
        module: usize,
        name: &str,
        value: bool,
        visiting: &mut Visiting,
    ) -> Option<(Def, Visibility)> {
        let scope = &self.crates[krate].modules[module].scope;
        for (_, imported, visibility) in scope.imports.iter().filter(|(bound, ..)| bound == name) {
            let found = self.path(krate, module, imported, value, visiting);
            // An import may bind the name only as a module or a type.
            if !(value && matches!(found, Def::Missing)) {
                return Some((found, *visibility));
            }
        }
        if let Some((_, what, visibility)) = scope.values.iter().find(|(value, ..)| value == name) {
            return Some((
                Def::Value(krate, module, name.to_owned(), what),
                *visibility,
            ));
        }
        let mut unknown = None;
        for (glob, through) in &scope.globs {
            match self.path(krate, module, glob, false, visiting) {
                Def::Module(source, inner) => {
                    // A glob imports only the names its module may use, and
                    // binds them no wider than the glob itself.
                    match self.member(source, inner, name, value, visiting) {
                        // An unknown name must not hide one a later glob imports.
                        Some((found @ (Def::Unknown(_) | Def::Missing), _)) => {
                            unknown = Some(found);
                        }
                        Some((found, Visibility::Public)) => return Some((found, *through)),
                        Some((found, Visibility::Within(outer)))
                            if source == krate && self.inside(krate, module, outer) =>
                        {
                            let narrower = match *through {
                                Visibility::Within(glob) if self.inside(krate, glob, outer) => glob,
                                _ => outer,
                            };
                            return Some((found, Visibility::Within(narrower)));
                        }
                        _ => {}
                    }
                }
                found @ (Def::Unknown(_) | Def::Missing) => unknown = Some(found),
                // A name a dependency's glob imports is outside the workspace,
                // as a name found nowhere is; a glob over an enum's variants
                // imports no type.
                Def::Outside
                | Def::Type(_)
                | Def::Trait(..)
                | Def::Value(..)
                | Def::NotAType(_) => {}
            }
        }
        // The source is read unexpanded, so a name an item macro may declare is
        // unknown rather than outside.
        if unknown.is_none() && scope.opaque {
            unknown = Some(Def::Unknown(
                "its module may declare it through an item macro or a file that was not read",
            ));
        }
        unknown.map(|unknown| (unknown, Visibility::Public))
    }

    /// Whether module `inner` of `krate` is `outer` or nested in it.
    fn inside(&self, krate: usize, inner: usize, outer: usize) -> bool {
        let modules = &self.crates[krate].modules;
        iter::successors(Some(inner), |&module| modules[module].parent)
            .any(|module| module == outer)
    }
}

type Visiting = Vec<(usize, usize, String)>;

#[cfg(test)]
mod tests {
    use super::{Def, Resolution, Resolver, TypeRef};
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
    fn a_glob_skips_a_name_its_module_may_not_use() {
        let crates = [library(
            "app",
            "mod hidden { struct Member; }
             mod shown { pub struct Member; }
             mod user { use crate::hidden::*; use crate::shown::*; }",
        )];
        assert_eq!(
            resolve(&crates, "user::", "Member"),
            named(&crates, 0, "shown::Member")
        );
    }

    #[test]
    fn a_glob_imports_the_names_a_restricted_visibility_lets_it_use() {
        let crates = [library(
            "app",
            "struct Root;
             mod outer {
                 pub mod hidden {
                     pub(super) struct Near; pub(in crate::outer) struct Named; pub(crate) struct Wide;
                 }
                 mod sibling { use super::hidden::*; }
                 mod tests { use crate::*; }
             }
             mod far { use crate::outer::hidden::*; }",
        )];
        for (module, target, name) in [
            ("outer::sibling::", "Near", Some("outer::hidden::Near")),
            ("outer::sibling::", "Named", Some("outer::hidden::Named")),
            ("outer::tests::", "Root", Some("Root")),
            ("far::", "Wide", Some("outer::hidden::Wide")),
            ("far::", "Near", None),
            ("far::", "Named", None),
        ] {
            let expected = name.map_or(Resolution::Outside, |name| named(&crates, 0, name));
            assert_eq!(
                resolve(&crates, module, target),
                expected,
                "{module}{target}"
            );
        }
    }

    #[test]
    fn a_glob_skips_the_private_imports_of_its_module() {
        let crates = [library(
            "app",
            "mod model { pub struct Member; pub struct Team; }
             mod middle { use crate::model::Member; use crate::model::*; }
             mod user { use crate::middle::*; }",
        )];
        for target in ["Member", "Team"] {
            assert_eq!(
                resolve(&crates, "user::", target),
                Resolution::Outside,
                "{target}"
            );
        }
    }

    #[test]
    fn workspace_libraries_resolve_by_crate_name() {
        let mut crates = [
            library("app", ""),
            library("model-types", "pub mod people { pub struct Member; }"),
        ];
        crates[0]
            .dependencies
            .push(("model_types".into(), "model-types".into()));
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
        let mut crates = [
            // One glob leaves the workspace and one cannot be followed.
            library("app", "use std::collections::*;\nuse self::generated::*;"),
            library("model-types", "pub struct Member;"),
        ];
        crates[0]
            .dependencies
            .push(("model_types".into(), "model-types".into()));
        assert_eq!(
            resolve(&crates, "", "model_types::Member"),
            named(&crates, 1, "Member")
        );
    }

    #[test]
    fn an_extern_crate_binds_a_workspace_library_under_its_rename() {
        let mut crates = [
            library("app", "extern crate model_types as mt;"),
            library("model-types", "pub mod people { pub struct Member; }"),
        ];
        crates[0]
            .dependencies
            .push(("model_types".into(), "model-types".into()));
        assert_eq!(
            resolve(&crates, "", "mt::people::Member"),
            named(&crates, 1, "people::Member")
        );
    }

    #[test]
    fn a_renamed_dependency_resolves_to_its_library() {
        let mut app = library("app", "");
        app.dependencies
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
    fn a_glob_into_a_module_with_an_item_macro_does_not_hide_a_later_glob() {
        let crates = [library(
            "app",
            "mod generated { declare_types!(Other); }
             mod model { pub struct Member; }
             use self::generated::*;
             use self::model::*;",
        )];
        assert_eq!(
            resolve(&crates, "", "Member"),
            named(&crates, 0, "model::Member")
        );
    }

    /// Whether `path`, as written in the root of the last of `crates`, names a
    /// function of the first crate's module at `module`.
    fn names_a_function(crates: &[Crate], path: &str, module: &str) -> bool {
        let path: syn::Path = syn::parse_str(path).expect("the path parses");
        let at = crates[0]
            .modules
            .iter()
            .position(|candidate| candidate.path == module)
            .expect("the module exists");
        matches!(
            Resolver::new(crates).find_value(crates.len() - 1, 0, &path),
            Def::Value(0, found, _, "a function") if found == at
        )
    }

    #[test]
    fn a_function_is_found_beside_a_module_of_its_name() {
        let crates = [library(
            "app",
            "mod checkout { pub fn checkout() {} } pub use checkout::checkout;",
        )];
        assert!(names_a_function(&crates, "checkout", "checkout::"));
    }

    #[test]
    fn a_function_is_found_through_a_library_of_its_name() {
        let mut crates = [
            library("checkout", "pub fn checkout() {}"),
            library("app", "pub use checkout::checkout;"),
        ];
        crates[1]
            .dependencies
            .push(("checkout".into(), "checkout".into()));
        assert!(names_a_function(&crates, "checkout", ""));
    }

    #[test]
    fn a_function_is_found_through_any_import_of_its_name() {
        let crates = [library(
            "app",
            "mod flows { pub mod checkout { pub fn checkout() {} } }
             pub use flows::checkout;
             pub use flows::checkout::checkout;",
        )];
        assert!(names_a_function(&crates, "checkout", "flows::checkout::"));
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
