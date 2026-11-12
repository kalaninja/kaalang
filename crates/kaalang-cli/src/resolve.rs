//! Resolves a `refer` target to the declaration it names, by Rust's path rules
//! over the workspace's module trees: `crate`, `self`, and `super`, declared
//! items, `use` imports with renames, groups, and globs, and the workspace's
//! libraries by their crate names. rustc has already checked that the path
//! names something; this finds what.

use std::{collections::BTreeMap, iter};

use syn::{
    ext::IdentExt,
    visit_mut::{self, VisitMut},
};

use crate::workspace::{Crate, Declared, UsePath, Visibility};

/// A type declaration: its crate and its index among the crate's types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TypeRef {
    pub(super) krate: usize,
    pub(super) id: usize,
}

impl TypeRef {
    /// Where the declaration is written: its file and its place in the file,
    /// which identify it across the crates that include the file.
    pub(super) fn place(self, crates: &[Crate]) -> (&std::path::Path, proc_macro2::LineColumn) {
        let krate = &crates[self.krate];
        let entry = &krate.types[self.id];
        (&krate.modules[entry.module].file.canonical, entry.position)
    }
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
    /// Several distinct declarations are visible through glob imports.
    Ambiguous,
    /// The source alone cannot tell, for the reason given.
    Unknown(&'static str),
}

/// What a path resolves to.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum Def {
    Type(TypeRef),
    Module(usize, usize),
    /// A trait: its crate, its module, and its name.
    Trait(usize, usize, String),
    /// An enum variant: its enum and name.
    Variant(TypeRef, String),
    /// A function, constant, or static: its crate, its module, its name, and
    /// what it is.
    Value(usize, usize, String, &'static str),
    NotAType(&'static str),
    Outside,
    /// Competing bindings in the same namespace, with their visibility in the scope.
    Ambiguous(Vec<(Def, Visibility)>),
    /// A module the walk read in full does not declare the name.
    Missing,
    Unknown(&'static str),
}

pub(super) struct Resolver<'a> {
    crates: &'a [Crate],
    /// Each crate's names for the workspace's libraries.
    externs: Vec<Vec<(String, usize)>>,
    /// One identity for each source declaration, including shared files.
    types: Vec<Vec<usize>>,
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
                        .filter(|(_, library)| library.package_id == *package)
                        .min_by_key(|(_, library)| library.host != krate.host)
                    {
                        names.push((name.clone(), *index));
                    }
                }
                // A package's other targets can name its own library.
                if !krate.library {
                    names.extend(
                        libraries
                            .iter()
                            .filter(|(_, library)| {
                                library.package_id == krate.package_id && library.host == krate.host
                            })
                            .map(|(index, library)| (library.name.clone(), *index)),
                    );
                }
                names
            })
            .collect();
        let mut declared = BTreeMap::new();
        let types = crates
            .iter()
            .enumerate()
            .map(|(index, krate)| {
                (0..krate.types.len())
                    .map(|id| {
                        let key = TypeRef { krate: index, id }.place(crates);
                        let next = declared.len();
                        *declared.entry(key).or_insert(next)
                    })
                    .collect()
            })
            .collect();
        Self {
            crates,
            externs,
            types,
        }
    }

    /// Resolves `path` as written in `module` of crate `krate`.
    pub(super) fn resolve(&self, krate: usize, module: usize, path: &syn::Path) -> Resolution {
        match self.find(krate, module, path) {
            Def::Type(found) => Resolution::Type(found),
            Def::Module(..) => Resolution::NotAType("a module"),
            Def::Trait(..) => Resolution::NotAType("a trait"),
            Def::Variant(..) => Resolution::NotAType("an enum variant"),
            Def::Value(.., what) | Def::NotAType(what) => Resolution::NotAType(what),
            Def::Outside => Resolution::Outside,
            Def::Ambiguous(_) => Resolution::Ambiguous,
            // rustc found it, so the walk missed it.
            Def::Missing => Resolution::Unknown("its module does not declare it as read"),
            Def::Unknown(reason) => Resolution::Unknown(reason),
        }
    }

    /// What `path`, as written in `module` of crate `krate`, names.
    pub(super) fn find(&self, krate: usize, module: usize, path: &syn::Path) -> Def {
        self.lookup(krate, module, path, false)
    }

    /// A query has not been checked by rustc: a bare missing name cannot use
    /// the fallback for external types unless its scope could provide it.
    pub(super) fn is_type(&self, krate: usize, module: usize, path: &syn::Path) -> Option<bool> {
        Some(match self.find(krate, module, path) {
            Def::Ambiguous(_) => return None,
            Def::Type(_) | Def::Unknown(_) => true,
            Def::Outside => {
                let Some(first) = path.segments.first() else {
                    return Some(false);
                };
                let name = first.ident.unraw().to_string();
                let external = matches!(name.as_str(), "std" | "core" | "alloc")
                    || self.crates[krate]
                        .dependencies
                        .iter()
                        .any(|(dependency, _)| dependency == &name);
                if path.segments.len() > 1 && external {
                    return Some(true);
                }
                if path.leading_colon.is_some() {
                    return Some(false);
                }
                let builtin = path.segments.len() == 1
                    && matches!(
                        name.as_str(),
                        "bool"
                            | "char"
                            | "str"
                            | "f16"
                            | "f32"
                            | "f64"
                            | "f128"
                            | "i8"
                            | "i16"
                            | "i32"
                            | "i64"
                            | "i128"
                            | "isize"
                            | "u8"
                            | "u16"
                            | "u32"
                            | "u64"
                            | "u128"
                            | "usize"
                            | "Box"
                            | "String"
                            | "Vec"
                            | "Option"
                            | "Result"
                    );
                builtin
                    || self
                        .member(krate, module, &name, false, &mut Lookup::default())
                        .is_some()
                    || self.crates[krate].modules[module]
                        .scope
                        .globs
                        .iter()
                        .any(|glob| {
                            matches!(
                                self.path(krate, module, &glob.0, false, &mut Lookup::default()),
                                Def::Outside
                            )
                        })
            }
            _ => false,
        })
    }

    /// What `path` names among functions, constants, and statics: its last
    /// segment passes over a module or type that shares the name, as in
    /// `mod checkout; pub use checkout::checkout;`.
    pub(super) fn find_value(&self, krate: usize, module: usize, path: &syn::Path) -> Def {
        self.lookup(krate, module, path, true)
    }

    /// Compares types after expanding aliases in their own scopes.
    pub(super) fn types_match(
        &self,
        query_scope: (usize, usize),
        query: &syn::Type,
        written_scope: (usize, usize),
        written: &syn::Type,
        generics: &syn::Generics,
    ) -> bool {
        let query = self.normalized_type(
            query_scope.0,
            query_scope.1,
            query,
            &syn::Generics::default(),
        );
        let written = self.normalized_type(written_scope.0, written_scope.1, written, generics);
        kaalang_compiler::types_match(&query, &written, generics, &mut |left, right| {
            let identity = |path: &syn::Path| {
                path.leading_colon
                    .and_then(|_| path.segments.first().map(|segment| segment.ident.clone()))
            };
            let last = |path: &syn::Path| path.segments.last().map(|segment| segment.ident.unraw());
            let (left_id, right_id) = (identity(left), identity(right));
            if left_id.as_ref().is_some_and(|id| id == "missing") {
                return false;
            }
            if left_id.as_ref().is_some_and(|id| id == "unknown")
                || right_id
                    .as_ref()
                    .is_some_and(|id| id == "unknown" || id == "missing")
            {
                return last(left) == last(right);
            }
            left_id == right_id && (left_id.is_some() || last(left) == last(right))
        })
    }

    /// Internal identities retain the authored name for unresolved-path fallback.
    fn normalized_type(
        &self,
        krate: usize,
        module: usize,
        ty: &syn::Type,
        generics: &syn::Generics,
    ) -> syn::Type {
        let mut ty = ty.clone();
        Normalize {
            resolver: self,
            krate,
            module,
            bound: generics
                .params
                .iter()
                .filter_map(|parameter| match parameter {
                    syn::GenericParam::Type(parameter) => Some(parameter.ident.clone()),
                    syn::GenericParam::Const(parameter) => Some(parameter.ident.clone()),
                    syn::GenericParam::Lifetime(_) => None,
                })
                .collect(),
            arguments: Vec::new(),
            aliases: Vec::new(),
        }
        .visit_type_mut(&mut ty);
        ty
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
        self.path(krate, module, &path, value, &mut Lookup::default())
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
        visiting: &mut Lookup,
    ) -> Def {
        let Some((first, rest)) = path.segments.split_first() else {
            return Def::Unknown("the path is empty");
        };
        let mut current = if path.global {
            self.prelude(krate, first).unwrap_or(Def::Outside)
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
                Def::Type(ty) => self.variant(ty, segment, value).map_or_else(
                    || {
                        if value {
                            Def::Missing
                        } else {
                            Def::NotAType("an associated item")
                        }
                    },
                    |(def, _)| def,
                ),
                Def::Trait(..) | Def::Value(..) | Def::Variant(..) | Def::NotAType(_) => {
                    Def::NotAType("an associated item")
                }
                outside_or_unknown => return outside_or_unknown,
            };
        }
        current
    }

    fn variant(&self, ty: TypeRef, name: &str, value: bool) -> Option<(Def, Visibility)> {
        let krate = &self.crates[ty.krate];
        let entry = &krate.types[ty.id];
        let syn::Item::Enum(enumeration) = &entry.item else {
            return None;
        };
        let variant = enumeration
            .variants
            .iter()
            .find(|variant| variant.ident.unraw() == name)?;
        if value && matches!(variant.fields, syn::Fields::Named(_)) {
            return None;
        }
        let (_, _, visibility) = krate.modules[entry.module].scope.declared.iter().find(
            |(_, declaration, _)| matches!(declaration, Declared::Type(id) if *id == ty.id),
        )?;
        Some((Def::Variant(ty, name.to_owned()), *visibility))
    }

    /// A relative path's first segment: a name in the module's scope, a
    /// workspace library, or a name from outside the workspace.
    fn first(
        &self,
        krate: usize,
        module: usize,
        name: &str,
        value: bool,
        visiting: &mut Lookup,
    ) -> Def {
        let library = || self.prelude(krate, name);
        match self
            .member(krate, module, name, value, visiting)
            .map(|(found, _)| found)
        {
            // A glob the walk could not follow may declare it, but a workspace
            // library of that name is what the path most likely means.
            Some(unknown @ (Def::Unknown(_) | Def::Missing)) => library().unwrap_or(unknown),
            Some(found @ Def::Value(..)) if !value => library().unwrap_or(found),
            Some(found) => found,
            None => library().unwrap_or(Def::Outside),
        }
    }

    /// What `name` stands for in the extern prelude, which every module may
    /// name: the crate a root `extern crate` binds to it, or a workspace
    /// library.
    fn prelude(&self, krate: usize, name: &str) -> Option<Def> {
        self.crates[krate].modules[0]
            .scope
            .declared
            .iter()
            .find_map(|(bound, declared, _)| match declared {
                Declared::ExternCrate(named) if bound == name => {
                    Some(self.extern_crate(krate, named))
                }
                _ => None,
            })
            .or_else(|| self.library(krate, name))
    }

    /// The crate root an `extern crate name` declaration names.
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
        visiting: &mut Lookup,
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
        let key = (krate, module, name.to_owned(), value);
        if let Some(found) = visiting.complete.get(&key) {
            return found.clone();
        }
        // Results truncated by a cycle depend on where the lookup started.
        if visiting.active.contains(&key) {
            visiting.cyclic = true;
            return None;
        }
        let cyclic = std::mem::take(&mut visiting.cyclic);
        visiting.active.push(key.clone());
        let found = self.member_unguarded(krate, module, name, value, visiting);
        visiting.active.pop();
        if !visiting.cyclic {
            visiting.complete.insert(key, found.clone());
        }
        visiting.cyclic |= cyclic;
        found
    }

    fn member_unguarded(
        &self,
        krate: usize,
        module: usize,
        name: &str,
        value: bool,
        visiting: &mut Lookup,
    ) -> Option<(Def, Visibility)> {
        let scope = &self.crates[krate].modules[module].scope;
        let mut local_value =
            scope
                .values
                .iter()
                .find(|(value, ..)| value == name)
                .map(|(_, what, visibility)| {
                    (
                        Def::Value(krate, module, name.to_owned(), what),
                        *visibility,
                    )
                });
        if value && local_value.is_some() {
            return local_value;
        }
        for (_, imported, visibility) in scope.imports.iter().filter(|(bound, ..)| bound == name) {
            match self.path(krate, module, imported, value, visiting) {
                // An import may bind the name only in the other namespace.
                Def::Missing if value => {}
                found @ Def::Value(..) if !value => {
                    local_value.get_or_insert((found, *visibility));
                }
                found => return Some((found, *visibility)),
            }
        }
        let mut unknown = None;
        let mut types = Vec::new();
        let mut values = Vec::new();
        for (glob, through) in &scope.globs {
            let (source, binding) = match self.path(krate, module, glob, false, visiting) {
                Def::Module(source, inner) => {
                    (source, self.member(source, inner, name, value, visiting))
                }
                Def::Type(ty) => (ty.krate, self.variant(ty, name, value)),
                found @ (Def::Unknown(_) | Def::Missing) => {
                    unknown = Some(found);
                    continue;
                }
                _ => continue,
            };
            // A glob imports only the names its module may use, and
            // binds them no wider than the glob itself.
            let mut add = |(found, visible)| {
                let visible = match visible {
                    Visibility::Public => *through,
                    Visibility::Within(outer)
                        if source == krate && self.inside(krate, module, outer) =>
                    {
                        match *through {
                            Visibility::Within(glob) if self.inside(krate, glob, outer) => *through,
                            _ => visible,
                        }
                    }
                    Visibility::Within(_) => return,
                };
                if matches!(found, Def::Unknown(_) | Def::Missing) {
                    unknown = Some(found);
                    return;
                }
                let bindings = if value || matches!(found, Def::Value(..)) {
                    &mut values
                } else {
                    &mut types
                };
                // Two paths to one declaration are one binding, with
                // the wider of their visibilities.
                if let Some((_, existing)) = bindings.iter_mut().find(|(def, _)| *def == found) {
                    if let Visibility::Within(narrower) = *existing
                        && match visible {
                            Visibility::Public => true,
                            Visibility::Within(wider) => self.inside(krate, narrower, wider),
                        }
                    {
                        *existing = visible;
                    }
                } else {
                    bindings.push((found, visible));
                }
            };
            match binding {
                Some((Def::Ambiguous(bindings), _)) => bindings.into_iter().for_each(add),
                Some(binding) => add(binding),
                None => {}
            }
        }
        // The source is read unexpanded, so a name an item macro may declare is
        // unknown rather than outside.
        if unknown.is_none() && scope.opaque {
            unknown = Some(Def::Unknown(
                "its module may declare it through an item macro or a file that was not read",
            ));
        }
        let binding = |mut bindings: Vec<_>| match bindings.len() {
            0 => None,
            1 => bindings.pop(),
            _ => Some((Def::Ambiguous(bindings), Visibility::Public)),
        };
        binding(types)
            .or(local_value)
            .or_else(|| binding(values))
            .or_else(|| unknown.map(|unknown| (unknown, Visibility::Public)))
    }

    /// Whether module `inner` of `krate` is `outer` or nested in it.
    fn inside(&self, krate: usize, inner: usize, outer: usize) -> bool {
        let modules = &self.crates[krate].modules;
        iter::successors(Some(inner), |&module| modules[module].parent)
            .any(|module| module == outer)
    }
}

type MemberKey = (usize, usize, String, bool);

/// Per lookup, so a shared glob re-export is read once even through many paths.
#[derive(Default)]
struct Lookup {
    active: Vec<MemberKey>,
    complete: BTreeMap<MemberKey, Option<(Def, Visibility)>>,
    cyclic: bool,
}

struct Normalize<'a, 'b> {
    resolver: &'a Resolver<'b>,
    krate: usize,
    module: usize,
    bound: Vec<syn::Ident>,
    arguments: Vec<(syn::Ident, syn::GenericArgument)>,
    aliases: Vec<TypeRef>,
}

impl Normalize<'_, '_> {
    fn argument(&self, path: &syn::Path) -> Option<&syn::GenericArgument> {
        let name = path.get_ident()?;
        self.arguments
            .iter()
            .find(|(parameter, _)| parameter == name)
            .map(|(_, argument)| argument)
    }

    fn alias(&mut self, path: &syn::Path) -> Option<syn::Type> {
        let Def::Type(found) = self.resolver.find(self.krate, self.module, path) else {
            return None;
        };
        let entry = &self.resolver.crates[found.krate].types[found.id];
        let syn::Item::Type(alias) = &entry.item else {
            return None;
        };
        if self.aliases.contains(&found) {
            return None;
        }
        let mut supplied = path.segments.last()?.arguments.clone();
        self.visit_path_arguments_mut(&mut supplied);
        let supplied = match supplied {
            syn::PathArguments::AngleBracketed(arguments) => arguments
                .args
                .into_iter()
                .filter(|argument| !matches!(argument, syn::GenericArgument::Lifetime(_)))
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let mut supplied = supplied.into_iter();
        let previous = (
            self.krate,
            self.module,
            std::mem::take(&mut self.bound),
            std::mem::take(&mut self.arguments),
        );
        self.krate = found.krate;
        self.module = entry.module;
        self.aliases.push(found);
        for parameter in &alias.generics.params {
            let (name, default) = match parameter {
                syn::GenericParam::Type(parameter) => (
                    &parameter.ident,
                    parameter
                        .default
                        .clone()
                        .map(|(_, ty)| syn::GenericArgument::Type(ty)),
                ),
                syn::GenericParam::Const(parameter) => (
                    &parameter.ident,
                    parameter
                        .default
                        .clone()
                        .map(|(_, expr)| syn::GenericArgument::Const(expr)),
                ),
                syn::GenericParam::Lifetime(_) => continue,
            };
            let argument = supplied.next().unwrap_or_else(|| {
                let mut default = default.unwrap_or_else(|| syn::parse_quote!(_));
                self.visit_generic_argument_mut(&mut default);
                default
            });
            self.arguments.push((name.clone(), argument));
        }
        let mut expanded = (*alias.ty).clone();
        self.visit_type_mut(&mut expanded);
        self.aliases.pop();
        (self.krate, self.module, self.bound, self.arguments) = previous;
        Some(expanded)
    }
}

impl VisitMut for Normalize<'_, '_> {
    fn visit_type_mut(&mut self, ty: &mut syn::Type) {
        if let syn::Type::Path(path) = ty
            && path.qself.is_none()
        {
            if let Some(syn::GenericArgument::Type(argument)) = self.argument(&path.path) {
                *ty = argument.clone();
                return;
            }
            if path
                .path
                .get_ident()
                .is_some_and(|name| self.bound.contains(name))
            {
                return;
            }
            if let Some(expanded) = self.alias(&path.path) {
                *ty = expanded;
                return;
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }

    fn visit_generic_argument_mut(&mut self, argument: &mut syn::GenericArgument) {
        if let syn::GenericArgument::Type(syn::Type::Path(path)) = argument
            && path.qself.is_none()
            && let Some(replacement) = self.argument(&path.path)
        {
            *argument = replacement.clone();
            return;
        }
        visit_mut::visit_generic_argument_mut(self, argument);
    }

    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Path(path) = expr {
            if path.qself.is_none()
                && let Some(argument) = self.argument(&path.path)
            {
                *expr = syn::parse_quote!(#argument);
                return;
            }
            // Constants are compared as written when their values are unknown.
            if !matches!(
                self.resolver.find(self.krate, self.module, &path.path),
                Def::Value(..)
            ) {
                for segment in &mut path.path.segments {
                    self.visit_path_arguments_mut(&mut segment.arguments);
                }
                return;
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_path_mut(&mut self, path: &mut syn::Path) {
        visit_mut::visit_path_mut(self, path);
        if path
            .get_ident()
            .is_some_and(|name| self.bound.contains(name))
        {
            return;
        }
        let identity = match self.resolver.find(self.krate, self.module, path) {
            Def::Type(found) => Some(format!(
                "type_{}",
                self.resolver.types[found.krate][found.id]
            )),
            Def::Trait(krate, module, name) => Some(format!("trait_{krate}_{module}_{name}")),
            Def::Value(krate, module, name, _) => Some(format!("value_{krate}_{module}_{name}")),
            Def::Unknown(_) => Some("unknown".into()),
            Def::Missing
            | Def::Module(..)
            | Def::Variant(..)
            | Def::NotAType(_)
            | Def::Ambiguous(_) => Some("missing".into()),
            Def::Outside => None,
        };
        let Some(last) = path.segments.last().cloned() else {
            return;
        };
        *path = match identity {
            Some(identity) => {
                let ident = syn::Ident::new(&identity, last.ident.span());
                syn::Path {
                    leading_colon: Some(syn::token::PathSep::default()),
                    segments: [syn::PathSegment::from(ident), last].into_iter().collect(),
                }
            }
            // Names outside the workspace retain the RFC's last-segment match.
            None => syn::Path {
                leading_colon: None,
                segments: iter::once(last).collect(),
            },
        };
    }
}

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
    fn a_root_extern_crate_is_named_from_every_module() {
        let mut app = library(
            "app",
            "extern crate self as this;
             extern crate model as domain;
             pub struct Report;
             mod review {}",
        );
        app.dependencies.push(("model".into(), "model".into()));
        let crates = [app, library("model", "pub struct Member;")];

        for (target, expected) in [
            ("this::Report", named(&crates, 0, "Report")),
            ("domain::Member", named(&crates, 1, "Member")),
            ("::domain::Member", named(&crates, 1, "Member")),
        ] {
            assert_eq!(resolve(&crates, "review::", target), expected, "{target}");
        }
    }

    #[test]
    fn query_types_reject_missing_names_but_keep_external_sources() {
        let mut krate = library(
            "app",
            "struct Report;
             mod nested {}
             trait Review {}
             const SIZE: usize = 2;
             use external::Report as Imported;
             mod local { pub use external::Report; }
             mod glob { use external::*; }
             mod generated { generate!(); }",
        );
        krate
            .dependencies
            .push(("external".into(), "external-package".into()));
        let crates = [krate];
        let resolver = Resolver::new(&crates);
        for (path, expected) in [
            ("Report", true),
            ("Imported", true),
            ("local::Report", true),
            ("external::Report", true),
            ("::external::Report", true),
            ("std::collections::HashMap", true),
            ("generated::Report", true),
            ("u8", true),
            ("Vec<u8>", true),
            ("Box<Report>", true),
            ("Missing", false),
            ("crate::Missing", false),
            ("unknown::Report", false),
            ("nested", false),
            ("Review", false),
            ("SIZE", false),
            ("::Imported", false),
        ] {
            assert_eq!(
                resolver.is_type(0, 0, &syn::parse_str(path).unwrap()),
                Some(expected),
                "{path}"
            );
        }
        let glob = crates[0]
            .modules
            .iter()
            .position(|module| module.path == "glob::")
            .unwrap();
        assert_eq!(
            resolver.is_type(0, glob, &syn::parse_quote!(ImportedByGlob)),
            Some(true)
        );
    }

    #[test]
    fn type_matching_preserves_resolved_and_unresolved_name_rules() {
        let crates = [library(
            "app",
            "struct Report;
             use Report as Renamed;
             mod generated { generate!(); }
             mod small { pub const SIZE: usize = 1; }
             mod large { pub const SIZE: usize = 2; }",
        )];
        let resolver = Resolver::new(&crates);
        for (query, written, matches) in [
            ("Renamed", "Report", true),
            ("generated::Report", "Report", true),
            ("Report", "generated::Report", true),
            ("dyn generated::Fn(u8)", "dyn Fn(u8)", true),
            ("external::Report", "Report", false),
            ("crate::String", "std::string::String", false),
            ("[u8; small::SIZE]", "[u8; small::SIZE]", true),
            ("[u8; small::SIZE]", "[u8; large::SIZE]", false),
            ("[u8; external::SMALL]", "[u8; other::SMALL]", false),
        ] {
            assert_eq!(
                resolver.types_match(
                    (0, 0),
                    &syn::parse_str(query).unwrap(),
                    (0, 0),
                    &syn::parse_str(written).unwrap(),
                    &syn::Generics::default(),
                ),
                matches,
                "{query} against {written}",
            );
        }
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
    fn a_glob_deduplicates_re_exports_of_one_type() {
        let crates = [library(
            "app",
            "mod model { pub struct Member; }
             mod left { pub use crate::model::Member; }
             mod right { pub use crate::model::Member; }
             use left::*;
             pub use right::*;
             mod user { use crate::*; }",
        )];
        for module in ["", "user::"] {
            assert_eq!(
                resolve(&crates, module, "Member"),
                named(&crates, 0, "model::Member")
            );
        }
    }

    #[test]
    fn value_names_do_not_hide_a_type_from_a_glob() {
        let crates = [library(
            "app",
            "mod model { pub struct Member { id: u8 } }
             mod functions { pub fn Member() {} }
             mod local { use crate::model::*; fn Member() {} }
             mod imported { use crate::model::*; use crate::functions::Member; }",
        )];
        for (module, declared) in [("local::", "local::"), ("imported::", "functions::")] {
            assert_eq!(
                resolve(&crates, module, "Member"),
                named(&crates, 0, "model::Member")
            );
            assert!(names_a_function(
                &crates,
                &format!("{module}Member"),
                declared
            ));
        }
    }

    #[test]
    fn visible_constructors_compete_with_functions_in_value_globs() {
        let crates = [library(
            "app",
            "mod functions {
                 pub fn Unit() {} pub fn Tuple() {} pub fn PrivateTuple() {}
                 pub fn UnitVariant() {} pub fn NamedVariant() {} pub fn PrivateVariant() {}
             }
             mod constructors {
                 pub struct Unit;
                 pub struct Tuple(pub u8);
                 pub struct PrivateTuple(u8);
                 pub enum Shape { UnitVariant, NamedVariant { value: u8 } }
                 pub use Shape::*;
                 enum Private { PrivateVariant }
                 pub use Private::*;
             }
             use functions::*;
             use constructors::*;",
        )];
        let resolver = Resolver::new(&crates);
        for name in ["Unit", "Tuple", "UnitVariant"] {
            assert!(
                matches!(
                    resolver.find_value(0, 0, &syn::parse_str(name).unwrap()),
                    Def::Ambiguous(_)
                ),
                "{name}"
            );
        }
        for name in ["PrivateTuple", "NamedVariant", "PrivateVariant"] {
            assert!(names_a_function(&crates, name, "functions::"), "{name}");
        }
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
            ("State::Open", "an enum variant"),
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
