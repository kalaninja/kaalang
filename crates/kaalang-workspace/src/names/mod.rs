//! Rust names and relationships in the active modules of a workspace target,
//! read from the module sources Cargo's targets compile.

use std::{io, iter, path::PathBuf, rc::Rc};

use kaalang_compiler::{CfgSet, SourceFile};
use syn::{Item, UseTree, ext::IdentExt};

pub use self::{
    resolve::{Def, Resolution, Resolver, TypeRef},
    select::{FlowPath, FoundFlow},
};
#[cfg(doc)]
use crate::target_group;
use crate::{
    CargoTarget, Files, module_items,
    walk::{TreeFile, walk},
};
mod resolve;
mod select;

/// A compiled crate and the libraries it can name.
pub struct Crate {
    pub package_id: String,
    pub package: String,
    /// The group its cards start a row in, as [`target_group`] gives it.
    pub group: u8,
    pub name: String,
    /// The target's own source path.
    pub root: PathBuf,
    pub library: bool,
    pub host: bool,
    pub dependencies: Vec<(String, String)>,
    pub cfg: CfgSet,
    pub modules: Vec<Module>,
    pub types: Vec<TypeEntry>,
}

/// One module: a file's own module or an inline module within one.
pub struct Module {
    /// The path from the crate root: `""`, or `review::inner::` with its
    /// trailing separator.
    pub path: String,
    pub parent: Option<usize>,
    pub file: Rc<File>,
    /// The path within its file: `""` for the file's own module.
    pub in_file: String,
    scope: Scope,
}

pub struct File {
    pub path: PathBuf,
    /// The path that tells two crates' copies of one file apart from two files.
    pub canonical: PathBuf,
    pub source: String,
    /// Its items as its crate's target compiles them, none when it could not
    /// be read.
    pub items: Vec<Item>,
}

/// The names a module declares and imports, compared without `r#`, each with
/// its visibility.
#[derive(Default)]
struct Scope {
    /// Declared names in the type namespace.
    pub declared: Vec<(String, Declared, Visibility)>,
    /// Functions, constants, and statics, with what each is.
    pub values: Vec<(String, &'static str, Visibility)>,
    /// Each name a `use` binds, with the path it imports.
    pub imports: Vec<(String, UsePath, Visibility)>,
    /// The module path of each glob import.
    pub globs: Vec<(UsePath, Visibility)>,
    /// Whether names this scope does not list may be declared here: by an
    /// item macro, or in a file that could not be read.
    pub opaque: bool,
}

/// Where a name may be used from: anywhere, or inside a module of its crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Within(usize),
}

enum Declared {
    Type(usize),
    Module(usize),
    Trait,
    /// `extern crate` with the crate it names; `self` names this crate.
    ExternCrate(String),
}

/// A path as written, its segments without `r#`.
struct UsePath {
    /// Whether it starts with `::`, naming a crate.
    pub global: bool,
    pub segments: Vec<String>,
}

/// A struct, enum, union, or alias declaration.
pub struct TypeEntry {
    pub module: usize,
    /// The path from the crate root, as `--type` names it.
    pub path: String,
    /// The path within its file, as `kaalang_compiler::type_items` lists it.
    pub in_file: String,
    /// The identifier's source position, unchanged by conditional attributes.
    pub position: proc_macro2::LineColumn,
    pub item: Item,
}

impl Crate {
    /// The Cargo target it compiles, as cards order it.
    #[must_use]
    pub fn target(&self) -> CargoTarget<'_> {
        CargoTarget {
            package: &self.package,
            group: self.group,
            name: &self.name,
            root: &self.root,
        }
    }
}

/// Modules and declarations in source order, with diagnostics from unreadable files.
#[derive(Default)]
pub struct ModuleTree {
    pub modules: Vec<Module>,
    pub types: Vec<TypeEntry>,
    pub skipped: Vec<String>,
}

/// A file of the tree: its text and what its target compiles of it.
type Read = TreeFile<(String, Vec<Item>)>;

impl ModuleTree {
    /// Reads the module tree whose crate root is `root`, as the target `cfg`
    /// compiles it.
    pub fn read(root: PathBuf, cfg: &CfgSet, files: &mut impl Files) -> Self {
        let root = walk(root, files, &mut |files, canonical, directory| {
            let source = files.read(canonical)?;
            let file = SourceFile::parse(&source, cfg).map_err(io::Error::other)?;
            let (modules, _) = module_items(&file, directory);
            let items = file.into_items();
            Ok(((source, items), modules))
        });
        let mut tree = Self::default();
        declare_file(&mut tree, root, String::new(), None);
        tree
    }
}

/// Declares the file's module at `path`, and what it declares. A file that
/// cannot be read or parsed is listed, and its module stays declared with an
/// opaque scope, so that a path into it is unknown rather than outside the
/// workspace.
fn declare_file(tree: &mut ModuleTree, file: Read, path: String, parent: Option<usize>) -> usize {
    let TreeFile {
        path: file_path,
        canonical,
        content,
        children,
        ..
    } = file;
    let (source, items, opaque) = match content {
        Ok((source, items)) => (source, items, false),
        Err(error) => {
            // A missing module file may be one a build script generates.
            if error.kind() != io::ErrorKind::NotFound {
                tree.skipped
                    .push(format!("{}: {error}", file_path.display()));
            }
            (String::new(), Vec::new(), true)
        }
    };
    let module = tree.modules.len();
    let shared = Rc::new(File {
        canonical: canonical.unwrap_or_else(|| file_path.clone()),
        path: file_path,
        source,
        items,
    });
    tree.modules.push(Module {
        path,
        parent,
        file: Rc::clone(&shared),
        in_file: String::new(),
        scope: Scope {
            opaque,
            ..Scope::default()
        },
    });
    declare_items(tree, &shared.items, module, &mut children.into_iter());
    module
}

/// Declares the names among `items` in `module` and follows its `mod` declarations.
fn declare_items(
    tree: &mut ModuleTree,
    items: &[Item],
    module: usize,
    children: &mut impl Iterator<Item = Option<Read>>,
) {
    for item in items {
        let name = |ident: &syn::Ident| ident.unraw().to_string();
        let (bound, declared, vis) = match item {
            Item::Mod(declaration) => {
                let Some(child) = declare_module(tree, declaration, module, children) else {
                    continue;
                };
                (
                    name(&declaration.ident),
                    Declared::Module(child),
                    &declaration.vis,
                )
            }
            Item::Struct(syn::ItemStruct { vis, ident, .. })
            | Item::Enum(syn::ItemEnum { vis, ident, .. })
            | Item::Union(syn::ItemUnion { vis, ident, .. })
            | Item::Type(syn::ItemType { vis, ident, .. }) => {
                if let Item::Struct(declaration) = item
                    && let Some(visible) =
                        constructor_visibility(&tree.modules, module, declaration)
                {
                    tree.modules[module].scope.values.push((
                        name(ident),
                        "a struct constructor",
                        visible,
                    ));
                }
                let outer = &tree.modules[module];
                tree.types.push(TypeEntry {
                    module,
                    path: format!("{}{ident}", outer.path),
                    in_file: format!("{}{ident}", outer.in_file),
                    position: ident.span().start(),
                    item: item.clone(),
                });
                (name(ident), Declared::Type(tree.types.len() - 1), vis)
            }
            Item::Trait(syn::ItemTrait { vis, ident, .. })
            | Item::TraitAlias(syn::ItemTraitAlias { vis, ident, .. }) => {
                (name(ident), Declared::Trait, vis)
            }
            Item::ExternCrate(declaration) => {
                let bound = declaration
                    .rename
                    .as_ref()
                    .map_or(&declaration.ident, |(_, rename)| rename);
                let named = name(&declaration.ident);
                (name(bound), Declared::ExternCrate(named), &declaration.vis)
            }
            Item::Fn(function) => {
                let visibility = visibility(&tree.modules, module, &function.vis);
                let value = (name(&function.sig.ident), "a function", visibility);
                tree.modules[module].scope.values.push(value);
                continue;
            }
            Item::Const(constant) => {
                let visibility = visibility(&tree.modules, module, &constant.vis);
                let value = (name(&constant.ident), "a constant", visibility);
                tree.modules[module].scope.values.push(value);
                continue;
            }
            Item::Static(staticdecl) => {
                let visibility = visibility(&tree.modules, module, &staticdecl.vis);
                let value = (name(&staticdecl.ident), "a static", visibility);
                tree.modules[module].scope.values.push(value);
                continue;
            }
            Item::Use(declaration) => {
                let visibility = visibility(&tree.modules, module, &declaration.vis);
                imports(
                    &declaration.tree,
                    &mut Vec::new(),
                    declaration.leading_colon.is_some(),
                    visibility,
                    &mut tree.modules[module].scope,
                );
                continue;
            }
            // An invocation may expand to items; a `macro_rules!` definition
            // has a name and declares none.
            Item::Macro(invocation) if invocation.ident.is_none() => {
                tree.modules[module].scope.opaque = true;
                continue;
            }
            _ => continue,
        };
        let visibility = visibility(&tree.modules, module, vis);
        tree.modules[module]
            .scope
            .declared
            .push((bound, declared, visibility));
    }
}

fn constructor_visibility(
    modules: &[Module],
    module: usize,
    declaration: &syn::ItemStruct,
) -> Option<Visibility> {
    let declared = visibility(modules, module, &declaration.vis);
    let fields = match &declaration.fields {
        syn::Fields::Unit => return Some(declared),
        syn::Fields::Unnamed(fields) => &fields.unnamed,
        syn::Fields::Named(_) => return None,
    };
    // The constructor is as private as its most restricted field. Restricted
    // scopes are ancestors of this module and are listed before their children.
    fields
        .iter()
        .map(|field| visibility(modules, module, &field.vis))
        .chain(iter::once(declared))
        .max_by_key(|visible| match visible {
            Visibility::Public => None,
            Visibility::Within(module) => Some(*module),
        })
}

/// Where an item of `module` with `vis` may be used from. A restricted
/// visibility names an ancestor of the module.
fn visibility(modules: &[Module], module: usize, vis: &syn::Visibility) -> Visibility {
    let restricted = match vis {
        syn::Visibility::Public(_) => return Visibility::Public,
        syn::Visibility::Inherited => return Visibility::Within(module),
        syn::Visibility::Restricted(restricted) => restricted,
    };
    let mut within = module;
    for segment in &restricted.path.segments {
        within = match segment.ident.to_string().as_str() {
            "crate" => 0,
            "self" => within,
            "super" => modules[within].parent.unwrap_or(0),
            name => {
                let path = format!("{}{name}::", modules[within].path);
                iter::successors(Some(module), |&ancestor| modules[ancestor].parent)
                    .find(|&ancestor| modules[ancestor].path == path)
                    .unwrap_or(0)
            }
        };
    }
    Visibility::Within(within)
}

/// Declares the module that `declaration` declares inside `parent`: an inline
/// one, or the next of the external modules the walk read, none for a file a
/// module above it already reads.
fn declare_module(
    tree: &mut ModuleTree,
    declaration: &syn::ItemMod,
    parent: usize,
    children: &mut impl Iterator<Item = Option<Read>>,
) -> Option<usize> {
    let path = format!("{}{}::", tree.modules[parent].path, declaration.ident);
    let Some((_, items)) = &declaration.content else {
        let file = children.next().flatten()?;
        return Some(declare_file(tree, file, path, Some(parent)));
    };
    let module = tree.modules.len();
    let outer = &tree.modules[parent];
    tree.modules.push(Module {
        path,
        parent: Some(parent),
        file: Rc::clone(&outer.file),
        in_file: format!("{}{}::", outer.in_file, declaration.ident),
        scope: Scope::default(),
    });
    declare_items(tree, items, module, children);
    Some(module)
}

/// Adds the names a `use` tree binds to `scope`, under the path `prefix` it
/// is nested in, with the visibility of the `use`.
fn imports(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    global: bool,
    visibility: Visibility,
    scope: &mut Scope,
) {
    let path = |prefix: &[String], last: &syn::Ident| {
        let mut segments = prefix.to_vec();
        // `a::{self}` imports `a` itself.
        if last != "self" {
            segments.push(last.unraw().to_string());
        }
        UsePath { global, segments }
    };
    match tree {
        UseTree::Path(nested) => {
            prefix.push(nested.ident.unraw().to_string());
            imports(&nested.tree, prefix, global, visibility, scope);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let imported = path(prefix, &name.ident);
            if let Some(bound) = imported.segments.last() {
                scope.imports.push((bound.clone(), imported, visibility));
            }
        }
        // `as _` binds no name.
        UseTree::Rename(rename) if rename.rename != "_" => {
            let bound = rename.rename.unraw().to_string();
            let imported = path(prefix, &rename.ident);
            scope.imports.push((bound, imported, visibility));
        }
        UseTree::Rename(_) => {}
        UseTree::Glob(_) => {
            let segments = prefix.clone();
            scope.globs.push((UsePath { global, segments }, visibility));
        }
        UseTree::Group(group) => {
            for tree in &group.items {
                imports(tree, prefix, global, visibility, scope);
            }
        }
    }
}

/// A library of one file, `source`, whose modules declared elsewhere are missing.
#[cfg(test)]
fn library(package: &str, source: &str) -> Crate {
    let root = PathBuf::from(format!("/{package}/src/lib.rs"));
    let cfg = CfgSet::default();
    let mut files = crate::walk::Memory(vec![(root.clone(), source.to_owned())]);
    let tree = ModuleTree::read(root, &cfg, &mut files);
    Crate {
        package_id: package.to_owned(),
        package: package.to_owned(),
        group: 0,
        name: package.replace('-', "_"),
        root: PathBuf::from(format!("/{package}/src/lib.rs")),
        library: true,
        host: false,
        dependencies: Vec::new(),
        cfg,
        modules: tree.modules,
        types: tree.types,
    }
}
