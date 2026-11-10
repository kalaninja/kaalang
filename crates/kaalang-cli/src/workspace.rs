//! Reads the current cargo workspace: each crate's module tree from its root,
//! with the names every module declares and imports, and its type
//! declarations in source order.

use std::{
    collections::{BTreeSet, HashMap},
    fmt::Write as _,
    fs, io, iter,
    path::{Path, PathBuf},
    rc::Rc,
};

use cargo_metadata::{MetadataCommand, Target};
use kaalang_compiler::{Declarations, FlowPath, Place, Question};
use kaalang_svg::RenderError;
use syn::{
    Attribute, Expr, ExprLit, Item, Lit, Meta, Token, UseTree, ext::IdentExt,
    punctuated::Punctuated, spanned::Spanned,
};

use crate::{
    datatype,
    resolve::{Def, Resolver},
};

/// The file that declares a found flow, its source, and the flow's position
/// among the file's flows.
pub(super) struct Found {
    pub(super) file: PathBuf,
    pub(super) source: String,
    pub(super) position: usize,
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
    /// Its items, none when it could not be read.
    pub(super) items: Vec<Item>,
}

/// The names a module declares and imports, compared without `r#`, each with
/// its visibility.
#[derive(Default)]
pub(super) struct Scope {
    /// Declared names in the type namespace.
    pub(super) declared: Vec<(String, Declared, Visibility)>,
    /// Functions, constants, and statics, with what each is.
    pub(super) values: Vec<(String, &'static str, Visibility)>,
    /// Each name a `use` binds, with the path it imports.
    pub(super) imports: Vec<(String, UsePath, Visibility)>,
    /// The module path of each glob import.
    pub(super) globs: Vec<(UsePath, Visibility)>,
    /// Whether names this scope does not list may be declared here: by an
    /// item macro, or in a file that could not be read.
    pub(super) opaque: bool,
}

/// Where a name may be used from: anywhere, or inside a module of its crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Visibility {
    Public,
    Within(usize),
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

/// Reads every target of every workspace member, in package name order, once
/// `package`, when given, is found among them.
pub(super) fn crates(package: Option<&str>) -> Result<Vec<Crate>, String> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|error| format!("could not read the cargo workspace: {error}"))?;
    let mut packages = metadata.workspace_packages();
    if let Some(package) = package
        && !packages.iter().any(|member| *member.name == package)
    {
        return Err(format!(
            "package `{package}` is not a member of this workspace"
        ));
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

/// Finds the one flow that `path` selects from its crate root, resolved as the
/// compiler resolves it, in every target of every workspace member, or only of
/// `package`. A crate's search spans its own files and those of the
/// workspace's libraries, as a trait's default method may be in one. A file
/// that cannot be read or parsed is skipped and listed, as an error when the
/// flow is not found and as a warning when it is.
pub(super) fn find(path: &str, package: Option<&str>) -> Result<Found, String> {
    let crates = crates(package)?;
    let resolver = Resolver::new(&crates);
    let selected = |krate: &Crate| package.is_none_or(|package| krate.package == package);
    // Each crate's files, by their own modules, with their declarations.
    let files: Vec<Vec<(usize, Declarations)>> = crates
        .iter()
        .map(|krate| {
            krate
                .modules
                .iter()
                .enumerate()
                .filter(|(_, module)| module.in_file.is_empty())
                .map(|(at, module)| (at, kaalang_compiler::declarations(&module.file.items)))
                .collect()
        })
        .collect();
    let mut matches = Vec::new();
    // A file that several crates include, or one crate reads twice, declares a
    // flow once.
    let mut seen = BTreeSet::new();
    let flow_path = FlowPath::parse(path);
    for (index, krate) in crates.iter().enumerate() {
        let Some(flow_path) = flow_path.as_ref().filter(|_| selected(krate)) else {
            continue;
        };
        let listed: Vec<((usize, usize), &Declarations)> = crates
            .iter()
            .enumerate()
            .filter(|(source, other)| *source == index || other.library)
            .flat_map(|(source, _)| {
                files[source]
                    .iter()
                    .map(move |(at, declarations)| ((source, *at), declarations))
            })
            .collect();
        let mut names =
            |question: Question<'_, (usize, usize)>| answer(&crates, index, &resolver, question);
        for (file, position) in kaalang_compiler::select(flow_path, &listed, &mut names) {
            let ((source, at), declarations) = listed[file];
            let module = &crates[source].modules[at];
            if seen.insert((module.file.canonical.clone(), position)) {
                let in_file = declarations.flows[position].path.clone();
                matches.push((source, module, in_file, position));
            }
        }
    }
    if matches.len() > 1 {
        let listed: Vec<(&Crate, String)> = matches
            .iter()
            .map(|(source, module, in_file, _)| {
                let at = format!("{}: {in_file}", module.file.path.display());
                (&crates[*source], at)
            })
            .collect();
        return Err(ambiguous(
            &RenderError::AmbiguousFlow(path.to_owned()),
            &listed,
        ));
    }
    // Only a crate the search spans could declare the flow.
    let skipped = skipped(
        crates
            .iter()
            .filter(|krate| selected(krate) || krate.library),
    );
    let Some((_, module, _, position)) = matches.pop() else {
        let message = RenderError::FlowNotFound(path.to_owned()).to_string();
        return Err(with_skipped(message, &skipped));
    };
    warn_skipped(path, &skipped);
    Ok(Found {
        file: module.file.path.clone(),
        source: module.file.source.clone(),
        position,
    })
}

/// Answers how names resolve for a flow path written from the root of crate
/// `krate`. A place names a crate and a file's own module in it.
fn answer(
    crates: &[Crate],
    krate: usize,
    resolver: &Resolver,
    question: Question<'_, (usize, usize)>,
) -> bool {
    // The module of a place: the file's own, or an inline one within it.
    let module = |at: Place<'_, (usize, usize)>| {
        let (source, own) = at.file;
        let modules = &crates[source].modules;
        let found = modules
            .iter()
            .position(|module| {
                Rc::ptr_eq(&module.file, &modules[own].file) && module.in_file == at.module
            })
            .unwrap_or(own);
        (source, found)
    };
    match question {
        Question::Function { path, at, name } => {
            let (source, inside) = module(at);
            matches!(
                resolver.find(krate, 0, path),
                Def::Value(found, declared, value, "a function")
                    if (found, declared) == (source, inside) && name.unraw() == value
            )
        }
        Question::Same(left, left_path, right, right_path) => {
            let last = |path: &syn::Path| path.segments.last().map(|segment| segment.ident.unraw());
            let (left_crate, left_module) = left.map_or((krate, 0), module);
            let (right_crate, right_module) = module(right);
            match (
                resolver.find(left_crate, left_module, left_path),
                resolver.find(right_crate, right_module, right_path),
            ) {
                // rustc has not checked the flow path, so a name it misses is wrong.
                (Def::Missing, _) if left.is_none() => false,
                (Def::Type(left), Def::Type(right)) => datatype::same(crates, left, right),
                (
                    Def::Trait(left, left_module, left_name),
                    Def::Trait(right, right_module, right_name),
                ) => (left, left_module, left_name) == (right, right_module, right_name),
                // The source alone cannot tell what a name it cannot resolve
                // is, nor which name from outside the workspace.
                (Def::Outside, Def::Outside)
                | (Def::Missing | Def::Unknown(_), _)
                | (_, Def::Missing | Def::Unknown(_)) => last(left_path) == last(right_path),
                _ => false,
            }
        }
    }
}

/// Lists where each match of a path is, after its crate, with a hint to `-p`
/// when a package has one match and could be selected.
pub(super) fn ambiguous(error: &dyn std::fmt::Display, matches: &[(&Crate, String)]) -> String {
    let mut message = error.to_string();
    message.push(':');
    for (krate, at) in matches {
        write!(message, "\n  {} in {at}", krate.label()).expect("writing to a String cannot fail");
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
pub(super) fn skipped<'a>(crates: impl IntoIterator<Item = &'a Crate>) -> BTreeSet<String> {
    crates
        .into_iter()
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
struct Walk {
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
    let readable = parsed.is_some();
    let shared = Rc::new(File {
        path: file.to_owned(),
        canonical: canonical.clone(),
        source,
        items: parsed.map(|parsed| parsed.items).unwrap_or_default(),
    });
    walk.modules.push(Module {
        path,
        parent,
        file: Rc::clone(&shared),
        in_file: String::new(),
        cfgs: Vec::new(),
        scope,
    });
    if readable {
        let base = file.parent().unwrap_or(Path::new(""));
        let directory = match file.file_stem() {
            Some(stem) if !owns_directory => base.join(stem),
            _ => base.to_owned(),
        };
        walk.open.push(canonical);
        walk_items(walk, &shared.items, module, &directory, base);
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
        let (bound, declared, vis) = match item {
            Item::Mod(declaration) => {
                let bound = name(&declaration.ident);
                let visibility = visibility(&walk.modules, module, &declaration.vis);
                for child in walk_module(walk, declaration, module, directory, base) {
                    let declared = (bound.clone(), Declared::Module(child), visibility);
                    walk.modules[module].scope.declared.push(declared);
                }
                continue;
            }
            Item::Struct(syn::ItemStruct {
                attrs, vis, ident, ..
            })
            | Item::Enum(syn::ItemEnum {
                attrs, vis, ident, ..
            })
            | Item::Union(syn::ItemUnion {
                attrs, vis, ident, ..
            })
            | Item::Type(syn::ItemType {
                attrs, vis, ident, ..
            }) => {
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
                (name(ident), Declared::Type(walk.types.len() - 1), vis)
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
                let visibility = visibility(&walk.modules, module, &function.vis);
                let value = (name(&function.sig.ident), "a function", visibility);
                walk.modules[module].scope.values.push(value);
                continue;
            }
            Item::Const(constant) => {
                let visibility = visibility(&walk.modules, module, &constant.vis);
                let value = (name(&constant.ident), "a constant", visibility);
                walk.modules[module].scope.values.push(value);
                continue;
            }
            Item::Static(staticdecl) => {
                let visibility = visibility(&walk.modules, module, &staticdecl.vis);
                let value = (name(&staticdecl.ident), "a static", visibility);
                walk.modules[module].scope.values.push(value);
                continue;
            }
            Item::Use(declaration) => {
                let visibility = visibility(&walk.modules, module, &declaration.vis);
                imports(
                    &declaration.tree,
                    &mut Vec::new(),
                    declaration.leading_colon.is_some(),
                    visibility,
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
        let visibility = visibility(&walk.modules, module, vis);
        walk.modules[module]
            .scope
            .declared
            .push((bound, declared, visibility));
    }
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

/// Reads the modules that `declaration` declares inside `parent` and returns
/// them: a module file is read once for its default file or `#[path]`, and
/// once for each `path` under `cfg_attr`.
fn walk_module(
    walk: &mut Walk,
    declaration: &syn::ItemMod,
    parent: usize,
    directory: &Path,
    base: &Path,
) -> Vec<usize> {
    let path = format!("{}{}::", walk.modules[parent].path, declaration.ident);
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
            vec![module]
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
            let conditional = cfg_attr_paths(&declaration.attrs);
            let mut files = Vec::new();
            // Under `cfg_attr` paths, the default file is only a fallback that
            // need not exist.
            if conditional.is_empty() || file.is_file() {
                files.push((file, owns_directory, Vec::new()));
            }
            for (attribute, conditions) in conditional {
                files.push((base.join(attribute), true, conditions));
            }
            files
                .into_iter()
                .filter_map(|(file, owns_directory, conditions)| {
                    let module =
                        walk_file(walk, &file, owns_directory, path.clone(), Some(parent))?;
                    let mut written = cfgs(&declaration.attrs);
                    written.extend(conditions);
                    walk.modules[module].cfgs = written;
                    Some(module)
                })
                .collect()
        }
    }
}

/// The `cfg` attributes among `attributes`, as written.
fn cfgs(attributes: &[Attribute]) -> Vec<String> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .filter_map(|attribute| as_written(&attribute.meta))
        .collect()
}

/// The source text of `tokens` with its whitespace collapsed.
fn as_written(tokens: &impl Spanned) -> Option<String> {
    let text = tokens.span().source_text()?;
    Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
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

/// The literal `#[path = "…"]` on a module declaration.
fn path_attribute(attributes: &[Attribute]) -> Option<String> {
    attributes
        .iter()
        .find_map(|attribute| path_value(&attribute.meta))
}

/// Each literal `path = "…"` under `cfg_attr` on a module declaration, with
/// the `cfg`s it applies under, outermost first.
fn cfg_attr_paths(attributes: &[Attribute]) -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    for attribute in attributes {
        if let Meta::List(list) = &attribute.meta
            && list.path.is_ident("cfg_attr")
        {
            cfg_attr_path(list, &mut Vec::new(), &mut found);
        }
    }
    found
}

fn cfg_attr_path(
    list: &syn::MetaList,
    conditions: &mut Vec<String>,
    found: &mut Vec<(String, Vec<String>)>,
) {
    let Ok(arguments) = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
    else {
        return;
    };
    let mut arguments = arguments.iter();
    let Some(predicate) = arguments.next().and_then(as_written) else {
        return;
    };
    conditions.push(format!("cfg({predicate})"));
    for argument in arguments {
        match argument {
            Meta::List(nested) if nested.path.is_ident("cfg_attr") => {
                cfg_attr_path(nested, conditions, found);
            }
            argument => {
                if let Some(path) = path_value(argument) {
                    found.push((path, conditions.clone()));
                }
            }
        }
    }
    conditions.pop();
}

/// The string of a `path = "…"` attribute.
fn path_value(meta: &Meta) -> Option<String> {
    match meta {
        Meta::NameValue(pair) if pair.path.is_ident("path") => match &pair.value {
            Expr::Lit(ExprLit {
                lit: Lit::Str(path),
                ..
            }) => Some(path.value()),
            _ => None,
        },
        _ => None,
    }
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
        open: Vec::new(),
        modules: vec![Module {
            path: String::new(),
            parent: None,
            file: Rc::new(File {
                path: PathBuf::from("src/lib.rs"),
                canonical: PathBuf::from(format!("/{package}/src/lib.rs")),
                source: source.to_owned(),
                items: parsed.items.clone(),
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
