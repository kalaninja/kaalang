//! Reads the current cargo workspace: each crate's module tree from its root,
//! with the names every module declares and imports, and its type
//! declarations in source order.

use std::{
    collections::{BTreeSet, HashMap},
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
};

use cargo_metadata::{MetadataCommand, Target};
use kaalang_svg::RenderError;
use syn::{Attribute, Expr, ExprLit, Item, Lit, Meta, UseTree, ext::IdentExt, spanned::Spanned};

/// The file that declares a found flow, its source, and the flow's path within it.
pub(super) struct Found {
    pub(super) file: PathBuf,
    pub(super) source: String,
    pub(super) path_in_file: String,
}

/// One crate: a package's target with the modules read from its root.
pub(super) struct Crate {
    pub(super) package: String,
    /// The name its code and other crates use for it.
    pub(super) name: String,
    /// The target as errors list it: `` lib `app` ``.
    pub(super) target: String,
    /// Where it stands among its package's crates: its library and binaries
    /// 0, then its tests 1, examples 2, and benches 3.
    pub(super) group: u8,
    /// Whether other crates may use it by its name.
    pub(super) library: bool,
    pub(super) root: PathBuf,
    /// The package's renamed dependencies: the name the code uses, and the
    /// package it names.
    pub(super) renames: Vec<(String, String)>,
    /// The modules in the order they were read; the first is the root.
    pub(super) modules: Vec<Module>,
    /// The struct, enum, union, and alias declarations in source order.
    pub(super) types: Vec<TypeEntry>,
    /// Each file that could not be read or parsed, with the reason.
    pub(super) skipped: Vec<String>,
}

impl Crate {
    /// The crate as an error lists it: its package and target.
    pub(super) fn label(&self) -> String {
        format!("{}: {}", self.package, self.target)
    }
}

/// One module: a file's own module or an inline module within one.
pub(super) struct Module {
    /// The path from the crate root: `""`, or `review::inner::` with its
    /// trailing separator.
    pub(super) path: String,
    pub(super) parent: Option<usize>,
    pub(super) file: Rc<File>,
    /// The path within its file: `""` for the file's own module.
    pub(super) in_file: String,
    /// The `cfg` attributes of the `mod` item that declares it, as written.
    pub(super) cfgs: Vec<String>,
    pub(super) scope: Scope,
}

pub(super) struct File {
    pub(super) path: PathBuf,
    /// The path that tells two crates' copies of one file apart from two files.
    pub(super) canonical: PathBuf,
    pub(super) source: String,
}

/// The names a module declares and imports, compared without `r#`.
#[derive(Default)]
pub(super) struct Scope {
    /// Declared names in the type namespace.
    pub(super) declared: Vec<(String, Declared)>,
    /// Functions, constants, and statics, with what each is.
    pub(super) values: Vec<(String, &'static str)>,
    /// Each name a `use` binds, with the path it imports.
    pub(super) imports: Vec<(String, UsePath)>,
    /// The module path of each glob import.
    pub(super) globs: Vec<UsePath>,
    /// Whether names this scope does not list may be declared here: by an
    /// item macro, or in a file that could not be read.
    pub(super) opaque: bool,
}

pub(super) enum Declared {
    Type(usize),
    Module(usize),
    Trait,
    /// `extern crate` with the crate it names; `self` names this crate.
    ExternCrate(String),
}

/// A path as written, its segments without `r#`.
pub(super) struct UsePath {
    /// Whether it starts with `::`, naming a crate.
    pub(super) global: bool,
    pub(super) segments: Vec<String>,
}

/// A struct, enum, union, or alias declaration.
pub(super) struct TypeEntry {
    pub(super) module: usize,
    /// The path from the crate root, as `--type` names it.
    pub(super) path: String,
    /// The path within its file, as `kaalang_compiler::type_items` lists it.
    pub(super) in_file: String,
    /// Which declaration of `in_file` in this reading of the file this is,
    /// counted from 0: exclusive `cfg`s may declare a path more than once.
    pub(super) occurrence: usize,
    /// The declaration's own `cfg` attributes, as written.
    pub(super) cfgs: Vec<String>,
    pub(super) item: Item,
}

/// Reads every target of every workspace member, or only of `package`, in
/// package name order. With `wanted`, only the modules that path passes
/// through are read.
pub(super) fn crates(wanted: Option<&str>, package: Option<&str>) -> Result<Vec<Crate>, String> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|error| format!("could not read the cargo workspace: {error}"))?;
    let mut packages = metadata.workspace_packages();
    if let Some(package) = package {
        packages.retain(|member| *member.name == package);
        if packages.is_empty() {
            return Err(format!(
                "package `{package}` is not a member of this workspace"
            ));
        }
    }
    packages.sort_by(|left, right| left.name.cmp(&right.name));

    let mut crates = Vec::new();
    for package in packages {
        let renames: Vec<(String, String)> = package
            .dependencies
            .iter()
            .filter_map(|dependency| {
                let rename = dependency.rename.as_ref()?;
                Some((rename.replace('-', "_"), dependency.name.clone()))
            })
            .collect();
        for target in package
            .targets
            .iter()
            .filter(|target| !target.is_custom_build())
        {
            let mut walk = Walk {
                wanted,
                open: Vec::new(),
                modules: Vec::new(),
                types: Vec::new(),
                occurrences: HashMap::new(),
                skipped: Vec::new(),
            };
            walk_file(
                &mut walk,
                target.src_path.as_std_path(),
                true,
                String::new(),
                None,
            );
            let group = if target.is_test() {
                1
            } else if target.is_example() {
                2
            } else if target.is_bench() {
                3
            } else {
                0
            };
            crates.push(Crate {
                package: package.name.to_string(),
                name: target.name.replace('-', "_"),
                target: target_label(target),
                group,
                library: group == 0 && !target.is_bin(),
                root: target.src_path.clone().into_std_path_buf(),
                renames: renames.clone(),
                modules: walk.modules,
                types: walk.types,
                skipped: walk.skipped,
            });
        }
    }
    Ok(crates)
}

/// Finds the one flow whose path from its crate root is `path`, searching every
/// target of every workspace member, or only of `package`. Only the modules
/// that the path passes through are read. A file that cannot be read or parsed
/// is skipped and listed, as an error when the flow is not found and as a
/// warning when it is.
pub(super) fn find(path: &str, package: Option<&str>) -> Result<Found, String> {
    let path = path.strip_prefix("crate::").unwrap_or(path);
    let crates = crates(Some(path), package)?;
    let mut matches = Vec::new();
    // A file that several crates include through `mod` declares a flow once.
    let mut seen = BTreeSet::new();
    for (index, krate) in crates.iter().enumerate() {
        // A file's flows are listed from its own module, with their inline
        // modules.
        for module in krate
            .modules
            .iter()
            .filter(|module| module.in_file.is_empty())
        {
            let source = &module.file.source;
            // Both `#[kaa]` and `#[kaalang::kaa]` contain the word, so most
            // files skip the second parse.
            if !source.contains("kaa") {
                continue;
            }
            for path_in_file in kaalang_svg::flow_names(source).expect("the source parsed") {
                if path.strip_prefix(&module.path) == Some(path_in_file.as_str()) {
                    if !seen.insert((module.file.canonical.clone(), path_in_file.clone())) {
                        continue;
                    }
                    let found = Found {
                        file: module.file.path.clone(),
                        source: source.clone(),
                        path_in_file,
                    };
                    matches.push((index, found));
                }
            }
        }
    }
    if matches.len() > 1 {
        let listed: Vec<(&Crate, &Path)> = matches
            .iter()
            .map(|(index, found)| (&crates[*index], found.file.as_path()))
            .collect();
        return Err(ambiguous(
            &RenderError::AmbiguousFlow(path.to_owned()),
            &listed,
        ));
    }
    let skipped = skipped(&crates);
    let Some((_, found)) = matches.pop() else {
        let message = RenderError::FlowNotFound(path.to_owned()).to_string();
        return Err(with_skipped(message, &skipped));
    };
    warn_skipped(path, &skipped);
    Ok(found)
}

/// Lists the crates that declare a path, with a hint to `-p` when a package
/// declares it once and could be selected.
pub(super) fn ambiguous(error: &dyn std::fmt::Display, matches: &[(&Crate, &Path)]) -> String {
    let mut message = error.to_string();
    message.push(':');
    for (krate, file) in matches {
        write!(message, "\n  {} in {}", krate.label(), file.display())
            .expect("writing to a String cannot fail");
    }
    let once = |package: &str| {
        matches
            .iter()
            .filter(|(other, _)| other.package == package)
            .count()
            == 1
    };
    if matches.iter().any(|(krate, _)| once(&krate.package)) {
        message.push_str("\nselect one with -p <PACKAGE>");
    }
    message
}

/// Every file the crates could not read, once each.
pub(super) fn skipped(crates: &[Crate]) -> BTreeSet<String> {
    crates
        .iter()
        .flat_map(|krate| krate.skipped.iter().cloned())
        .collect()
}

/// Warns that `path` may also be declared in a file that was skipped.
pub(super) fn warn_skipped(path: &str, skipped: &BTreeSet<String>) {
    if !skipped.is_empty() {
        let warning = format!("warning: `{path}` may also be declared in a file that was skipped");
        eprintln!("{}: {}", crate::BIN_NAME, with_skipped(warning, skipped));
    }
}

/// Appends the files the walk could not search to `message`.
pub(super) fn with_skipped(mut message: String, skipped: &BTreeSet<String>) -> String {
    if !skipped.is_empty() {
        message.push_str("\nthese files and their submodules could not be searched:");
        for file in skipped {
            write!(message, "\n  {file}").expect("writing to a String cannot fail");
        }
    }
    message
}

/// One crate's module tree as it is read.
struct Walk<'a> {
    /// The path whose modules alone are read, if any.
    wanted: Option<&'a str>,
    /// The files being read, outermost first; a `#[path]` loop stops here.
    open: Vec<PathBuf>,
    modules: Vec<Module>,
    types: Vec<TypeEntry>,
    /// How many declarations of each path each reading of a file has had.
    occurrences: HashMap<(*const File, String), usize>,
    skipped: Vec<String>,
}

/// Reads the file of the module at `path` and follows its `mod` declarations,
/// returning the module. A crate root, a `mod.rs`, and a `#[path]` file own
/// their directory; another file `a.rs` keeps its submodules in `a/`. A file
/// that cannot be read or parsed is skipped and listed, and its module stays
/// declared with an opaque scope, so that a path into it is unknown rather than
/// outside the workspace.
fn walk_file(
    walk: &mut Walk,
    file: &Path,
    owns_directory: bool,
    path: String,
    parent: Option<usize>,
) -> Option<usize> {
    let (canonical, source, parsed, scope) = match read(file) {
        Ok((canonical, source, parsed)) => (canonical, source, Some(parsed), Scope::default()),
        Err(error) => {
            // A missing module file may be generated or belong to another
            // configuration.
            if error.kind() != io::ErrorKind::NotFound {
                walk.skipped.push(format!("{}: {error}", file.display()));
            }
            let opaque = Scope {
                opaque: true,
                ..Scope::default()
            };
            (file.to_owned(), String::new(), None, opaque)
        }
    };
    if walk.open.contains(&canonical) {
        return None;
    }
    let module = walk.modules.len();
    walk.modules.push(Module {
        path,
        parent,
        file: Rc::new(File {
            path: file.to_owned(),
            canonical: canonical.clone(),
            source,
        }),
        in_file: String::new(),
        cfgs: Vec::new(),
        scope,
    });
    if let Some(parsed) = parsed {
        let base = file.parent().unwrap_or(Path::new(""));
        let directory = match file.file_stem() {
            Some(stem) if !owns_directory => base.join(stem),
            _ => base.to_owned(),
        };
        walk.open.push(canonical);
        walk_items(walk, &parsed.items, module, &directory, base);
        walk.open.pop();
    }
    Some(module)
}

/// A module file's canonical path, source, and items.
fn read(file: &Path) -> io::Result<(PathBuf, String, syn::File)> {
    let canonical = fs::canonicalize(file)?;
    let source = fs::read_to_string(file)?;
    let parsed = syn::parse_file(&source).map_err(io::Error::other)?;
    Ok((canonical, source, parsed))
}

/// Declares the names among `items` in `module` and follows its `mod`
/// declarations. `directory` holds their default files, and a `#[path]` is
/// relative to `base`.
fn walk_items(walk: &mut Walk, items: &[Item], module: usize, directory: &Path, base: &Path) {
    for item in items {
        let name = |ident: &syn::Ident| ident.unraw().to_string();
        let declared = match item {
            Item::Mod(declaration) => {
                let Some(child) = walk_module(walk, declaration, module, directory, base) else {
                    continue;
                };
                (name(&declaration.ident), Declared::Module(child))
            }
            Item::Struct(syn::ItemStruct { attrs, ident, .. })
            | Item::Enum(syn::ItemEnum { attrs, ident, .. })
            | Item::Union(syn::ItemUnion { attrs, ident, .. })
            | Item::Type(syn::ItemType { attrs, ident, .. }) => {
                let outer = &walk.modules[module];
                let (path, in_file) = (
                    format!("{}{ident}", outer.path),
                    format!("{}{ident}", outer.in_file),
                );
                let reading = Rc::as_ptr(&outer.file);
                let count = walk
                    .occurrences
                    .entry((reading, in_file.clone()))
                    .or_default();
                let occurrence = *count;
                *count += 1;
                walk.types.push(TypeEntry {
                    module,
                    path,
                    in_file,
                    occurrence,
                    cfgs: cfgs(attrs),
                    item: item.clone(),
                });
                (name(ident), Declared::Type(walk.types.len() - 1))
            }
            Item::Trait(syn::ItemTrait { ident, .. })
            | Item::TraitAlias(syn::ItemTraitAlias { ident, .. }) => (name(ident), Declared::Trait),
            Item::ExternCrate(declaration) => {
                let bound = declaration
                    .rename
                    .as_ref()
                    .map_or(&declaration.ident, |(_, rename)| rename);
                let named = name(&declaration.ident);
                (name(bound), Declared::ExternCrate(named))
            }
            Item::Fn(function) => {
                let value = (name(&function.sig.ident), "a function");
                walk.modules[module].scope.values.push(value);
                continue;
            }
            Item::Const(constant) => {
                let value = (name(&constant.ident), "a constant");
                walk.modules[module].scope.values.push(value);
                continue;
            }
            Item::Static(staticdecl) => {
                let value = (name(&staticdecl.ident), "a static");
                walk.modules[module].scope.values.push(value);
                continue;
            }
            Item::Use(declaration) => {
                let mut prefix = Vec::new();
                let global = declaration.leading_colon.is_some();
                imports(
                    &declaration.tree,
                    &mut prefix,
                    global,
                    &mut walk.modules[module].scope,
                );
                continue;
            }
            // An invocation may expand to items; a `macro_rules!` definition
            // has a name and declares none.
            Item::Macro(invocation) if invocation.ident.is_none() => {
                walk.modules[module].scope.opaque = true;
                continue;
            }
            _ => continue,
        };
        walk.modules[module].scope.declared.push(declared);
    }
}

/// Reads the module that `declaration` declares inside `parent`, unless it is
/// off the wanted path, and returns it.
fn walk_module(
    walk: &mut Walk,
    declaration: &syn::ItemMod,
    parent: usize,
    directory: &Path,
    base: &Path,
) -> Option<usize> {
    let path = format!("{}{}::", walk.modules[parent].path, declaration.ident);
    // Every flow inside the module has a path that starts with the module's.
    if walk.wanted.is_some_and(|wanted| !wanted.starts_with(&path)) {
        return None;
    }
    let name = declaration.ident.unraw().to_string();
    let attribute = path_attribute(&declaration.attrs);
    match (&declaration.content, attribute) {
        (Some((_, items)), attribute) => {
            // An inline module's `#[path]` names its directory.
            let directory =
                attribute.map_or_else(|| directory.join(&name), |attribute| base.join(attribute));
            let module = walk.modules.len();
            let outer = &walk.modules[parent];
            walk.modules.push(Module {
                path,
                parent: Some(parent),
                file: Rc::clone(&outer.file),
                in_file: format!("{}{}::", outer.in_file, declaration.ident),
                cfgs: cfgs(&declaration.attrs),
                scope: Scope::default(),
            });
            walk_items(walk, items, module, &directory, &directory);
            Some(module)
        }
        (None, attribute) => {
            let (file, owns_directory) = if let Some(attribute) = attribute {
                (base.join(attribute), true)
            } else {
                let file = directory.join(format!("{name}.rs"));
                if file.is_file() {
                    (file, false)
                } else {
                    (directory.join(&name).join("mod.rs"), true)
                }
            };
            let module = walk_file(walk, &file, owns_directory, path, Some(parent))?;
            walk.modules[module].cfgs = cfgs(&declaration.attrs);
            Some(module)
        }
    }
}

/// The `cfg` attributes among `attributes`, as written.
fn cfgs(attributes: &[Attribute]) -> Vec<String> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .filter_map(|attribute| attribute.meta.span().source_text())
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// Adds the names a `use` tree binds to `scope`, under the path `prefix` it
/// is nested in.
fn imports(tree: &UseTree, prefix: &mut Vec<String>, global: bool, scope: &mut Scope) {
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
            imports(&nested.tree, prefix, global, scope);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let imported = path(prefix, &name.ident);
            if let Some(bound) = imported.segments.last() {
                scope.imports.push((bound.clone(), imported));
            }
        }
        // `as _` binds no name.
        UseTree::Rename(rename) if rename.rename != "_" => {
            let bound = rename.rename.unraw().to_string();
            scope.imports.push((bound, path(prefix, &rename.ident)));
        }
        UseTree::Rename(_) => {}
        UseTree::Glob(_) => scope.globs.push(UsePath {
            global,
            segments: prefix.clone(),
        }),
        UseTree::Group(group) => {
            for tree in &group.items {
                imports(tree, prefix, global, scope);
            }
        }
    }
}

/// The literal `#[path = "…"]` on a module declaration.
// shortcut: a `path` inside `cfg_attr` is not read, so that module is skipped; upgrade by walking every `cfg_attr` path, as rustfmt does.
fn path_attribute(attributes: &[Attribute]) -> Option<String> {
    attributes
        .iter()
        .find_map(|attribute| match &attribute.meta {
            Meta::NameValue(pair) if pair.path.is_ident("path") => match &pair.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(path),
                    ..
                }) => Some(path.value()),
                _ => None,
            },
            _ => None,
        })
}

/// Names a crate by its target, as `` lib `app` `` or `` test `cli` ``.
fn target_label(target: &Target) -> String {
    let kinds: Vec<String> = target.kind.iter().map(ToString::to_string).collect();
    format!("{} `{}`", kinds.join(", "), target.name)
}

/// The library of `package`, read from `source` without cargo: its modules
/// must be inline.
#[cfg(test)]
pub(super) fn library(package: &str, source: &str) -> Crate {
    let parsed = syn::parse_file(source).expect("the test source parses");
    let mut walk = Walk {
        wanted: None,
        open: Vec::new(),
        modules: vec![Module {
            path: String::new(),
            parent: None,
            file: Rc::new(File {
                path: PathBuf::from("src/lib.rs"),
                canonical: PathBuf::from(format!("/{package}/src/lib.rs")),
                source: source.to_owned(),
            }),
            in_file: String::new(),
            cfgs: Vec::new(),
            scope: Scope::default(),
        }],
        types: Vec::new(),
        occurrences: HashMap::new(),
        skipped: Vec::new(),
    };
    walk_items(
        &mut walk,
        &parsed.items,
        0,
        Path::new("src"),
        Path::new("src"),
    );
    Crate {
        package: package.to_owned(),
        name: package.replace('-', "_"),
        target: format!("lib `{package}`"),
        group: 0,
        library: true,
        root: PathBuf::from(format!("/{package}/src/lib.rs")),
        renames: Vec::new(),
        modules: walk.modules,
        types: walk.types,
        skipped: walk.skipped,
    }
}
