//! Reads the current cargo workspace: each crate's module tree from its root,
//! with the names every module declares and imports, and its type
//! declarations in source order.

use std::{
    collections::BTreeSet,
    env,
    fmt::Write as _,
    fs, io, iter,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

use cargo_metadata::{CargoOpt, DependencyKind, MetadataCommand, Target};
use kaalang_compiler::{Declarations, FlowPath, Place, Question};
use kaalang_svg::RenderError;
use syn::{Attribute, Expr, ExprLit, Item, Lit, Meta, UseTree, ext::IdentExt};

use crate::{
    datatype,
    resolve::{Def, Resolver},
};

/// The file that declares a found flow, its source, the flow's position among
/// the file's flows, and the cfg set its crate is read for.
pub(super) struct Found {
    pub(super) file: PathBuf,
    pub(super) source: String,
    pub(super) position: usize,
    pub(super) cfg: kaalang_svg::Target,
}

/// The target options, as cargo takes them.
#[derive(clap::Args)]
pub(super) struct TargetOptions {
    /// Target triple to read the code for [default: the host's]
    #[arg(long)]
    pub(super) target: Option<String>,
    /// Features to enable, as cargo takes them
    #[arg(long)]
    pub(super) features: Vec<String>,
    /// Enable every feature
    #[arg(long)]
    pub(super) all_features: bool,
    /// Leave out the default features
    #[arg(long)]
    pub(super) no_default_features: bool,
}

/// One crate: a package's target with the modules read from its root.
pub(super) struct Crate {
    pub(super) package: String,
    pub(super) package_id: String,
    /// The name its code and other crates use for it.
    pub(super) name: String,
    /// The target as errors list it: `` lib `app` ``.
    pub(super) target: String,
    /// Where it stands among its package's crates: its library and binaries
    /// 0, then its tests 1, examples 2, and benches 3.
    pub(super) group: u8,
    /// Whether other crates may use it by its name.
    pub(super) library: bool,
    /// Read for the host during cross-compilation.
    pub(super) host: bool,
    /// An additional host reading of a library, reached through dependencies only.
    pub(super) dependency_only: bool,
    pub(super) root: PathBuf,
    /// Dependency names and resolved package IDs available to this target.
    pub(super) dependencies: Vec<(String, String)>,
    /// The modules in the order they were read; the first is the root.
    pub(super) modules: Vec<Module>,
    /// The struct, enum, union, and alias declarations in source order.
    pub(super) types: Vec<TypeEntry>,
    /// Each file that could not be read or parsed, with the reason.
    pub(super) skipped: Vec<String>,
    /// The cfg set its files are read for.
    pub(super) cfg: kaalang_svg::Target,
}

impl Crate {
    /// The crate as an error lists it.
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
    pub(super) scope: Scope,
}

pub(super) struct File {
    pub(super) path: PathBuf,
    /// The path that tells two crates' copies of one file apart from two files.
    pub(super) canonical: PathBuf,
    pub(super) source: String,
    /// Its items as its crate's target compiles them, none when it could not
    /// be read.
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
    /// The identifier's source position, unchanged by conditional attributes.
    pub(super) position: proc_macro2::LineColumn,
    pub(super) item: Item,
}

/// Reads every target of every workspace member, in package name order, once
/// `package`, when given, is found among them, each as cargo compiles it for
/// the target `options` name.
pub(super) fn crates(package: Option<&str>, options: &TargetOptions) -> Result<Vec<Crate>, String> {
    let printed = rustc_cfg(options.target.as_deref(), true)?;
    let metadata = metadata(package, options, options.target.as_deref())?;
    let mut packages = metadata.workspace_packages();
    if let Some(package) = package
        && !packages.iter().any(|member| *member.name == package)
    {
        return Err(format!(
            "package `{package}` is not a member of this workspace"
        ));
    }
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    // Cargo builds a procedural macro for the host, without the target's flags.
    let host = if options.target.is_some()
        && packages
            .iter()
            .any(|package| package.targets.iter().any(Target::is_proc_macro))
    {
        Some((
            rustc_cfg(None, false)?,
            self::metadata(package, options, None)?,
        ))
    } else {
        None
    };

    let host_dependencies = host
        .as_ref()
        .map_or_else(BTreeSet::new, |(_, metadata)| host_dependencies(metadata));
    let mut crates = Vec::new();
    for package in packages {
        let targets = package
            .targets
            .iter()
            .filter(|target| !target.is_custom_build());
        for (target, dependency_only) in targets.flat_map(|target| {
            let also_host = (target.is_lib() || target.is_rlib() || target.is_dylib())
                && host_dependencies.contains(&package.id);
            iter::once((target, false)).chain(also_host.then_some((target, true)))
        }) {
            let for_host = host.is_some() && (target.is_proc_macro() || dependency_only);
            let (printed, metadata) = if for_host {
                let (printed, metadata) = host.as_ref().expect("the host was read");
                (printed, metadata)
            } else {
                (&printed, &metadata)
            };
            let resolved = metadata
                .resolve
                .as_ref()
                .and_then(|resolve| resolve.nodes.iter().find(|node| node.id == package.id));
            // Cargo builds no target whose required features are off.
            if !required_features(&target.required_features, package, resolved, metadata) {
                continue;
            }
            let features: Vec<String> = resolved
                .map(|node| node.features.iter().map(ToString::to_string).collect())
                .unwrap_or_default();
            let cfg = cfg_set(target, printed, &features)?;
            let mut walk = Walk::new(&cfg);
            walk_file(
                &mut walk,
                target.src_path.as_std_path(),
                true,
                String::new(),
                None,
            );
            let group = target_group(target);
            crates.push(Crate {
                package: package.name.to_string(),
                package_id: package.id.to_string(),
                name: target.name.replace('-', "_"),
                target: target_label(target),
                group,
                library: group == 0 && !target.is_bin(),
                host: for_host,
                dependency_only,
                root: target.src_path.clone().into_std_path_buf(),
                dependencies: resolved
                    .into_iter()
                    .flat_map(|node| &node.deps)
                    .filter(|dependency| {
                        dependency.dep_kinds.iter().any(|kind| {
                            kind.kind == DependencyKind::Normal
                                || kind.kind == DependencyKind::Development && group != 0
                        })
                    })
                    .map(|dependency| (dependency.name.clone(), dependency.pkg.to_string()))
                    .collect(),
                modules: walk.modules,
                types: walk.types,
                skipped: walk.skipped,
                cfg,
            });
        }
    }
    Ok(crates)
}

/// Libraries a procedural macro reaches through normal dependencies run on the host too.
fn host_dependencies(metadata: &cargo_metadata::Metadata) -> BTreeSet<cargo_metadata::PackageId> {
    let mut pending: Vec<_> = metadata
        .workspace_packages()
        .into_iter()
        .filter(|package| package.targets.iter().any(Target::is_proc_macro))
        .map(|package| package.id.clone())
        .collect();
    let mut found = BTreeSet::new();
    while let Some(package) = pending.pop() {
        if !found.insert(package.clone()) {
            continue;
        }
        if let Some(node) = metadata
            .resolve
            .as_ref()
            .and_then(|resolve| resolve.nodes.iter().find(|node| node.id == package))
        {
            pending.extend(
                node.deps
                    .iter()
                    .filter(|dependency| {
                        dependency
                            .dep_kinds
                            .iter()
                            .any(|kind| kind.kind == DependencyKind::Normal)
                    })
                    .map(|dependency| dependency.pkg.clone()),
            );
        }
    }
    found
}

/// Whether every local or dependency-qualified requirement is enabled.
fn required_features(
    required: &[String],
    package: &cargo_metadata::Package,
    resolved: Option<&cargo_metadata::Node>,
    metadata: &cargo_metadata::Metadata,
) -> bool {
    let Some(node) = resolved else {
        return required.is_empty();
    };
    required.iter().all(|required| {
        let Some((name, feature)) = required.split_once('/') else {
            return node.features.iter().any(|enabled| enabled == required);
        };
        node.deps.iter().any(|edge| {
            let dependency = &metadata[&edge.pkg];
            let named = package.dependencies.iter().any(|declared| {
                declared.rename.as_ref().unwrap_or(&declared.name) == name
                    && dependency.name == declared.name
                    && declared.rename.as_ref().map_or_else(
                        || {
                            dependency.targets.iter().any(|target| {
                                (target.is_lib()
                                    || target.is_rlib()
                                    || target.is_dylib()
                                    || target.is_proc_macro())
                                    && target.name.replace('-', "_") == edge.name
                            })
                        },
                        |rename| rename.replace('-', "_") == edge.name,
                    )
            });
            named
                && metadata
                    .resolve
                    .as_ref()
                    .and_then(|resolve| resolve.nodes.iter().find(|node| node.id == edge.pkg))
                    .is_some_and(|node| node.features.iter().any(|enabled| *enabled == feature))
        })
    })
}

/// The cfg set cargo builds `target` with: what rustc `printed` for its
/// compilation target, its package's `features`, `test` for a test or bench
/// target, which cargo builds with `--test`, and `proc_macro`, which rustc sets
/// for a procedural macro.
fn cfg_set(
    target: &Target,
    printed: &str,
    features: &[String],
) -> Result<kaalang_svg::Target, String> {
    let mut lines = vec![printed.to_owned()];
    lines.extend(
        features
            .iter()
            .map(|feature| format!("feature=\"{feature}\"")),
    );
    if target.is_test() || target.is_bench() {
        lines.push("test".to_owned());
    }
    if target.is_proc_macro() {
        lines.push("proc_macro".to_owned());
    }
    lines
        .join("\n")
        .parse()
        .map_err(|error| format!("could not read the cfg rustc printed: {error}"))
}

/// The workspace as `cargo metadata` describes it for `target`, or the host,
/// with the features `options` select. As with
/// `cargo build -p`, a feature without a package is `package`'s.
fn metadata(
    package: Option<&str>,
    options: &TargetOptions,
    target: Option<&str>,
) -> Result<cargo_metadata::Metadata, String> {
    let triple = match target {
        Some(triple) => triple.to_owned(),
        None => host()?,
    };
    let mut command = MetadataCommand::new();
    command.other_options(vec!["--filter-platform".to_owned(), triple]);
    if options.all_features {
        command.features(CargoOpt::AllFeatures);
    }
    if options.no_default_features {
        command.features(CargoOpt::NoDefaultFeatures);
    }
    // Cargo takes features separated by commas or spaces.
    let features: Vec<String> = options
        .features
        .iter()
        .flat_map(|features| features.split([',', ' ']))
        .filter(|feature| !feature.is_empty())
        .map(|feature| match package {
            Some(package) if !feature.contains('/') => format!("{package}/{feature}"),
            _ => feature.to_owned(),
        })
        .collect();
    if !features.is_empty() {
        command.features(CargoOpt::SomeFeatures(features));
    }
    command
        .exec()
        .map_err(|error| format!("could not read the cargo workspace: {error}"))
}

/// What `rustc --print cfg` prints for `target`, or the host, with the flags
/// cargo passes when `flags` is set.
fn rustc_cfg(target: Option<&str>, flags: bool) -> Result<String, String> {
    let mut arguments = vec!["--print".to_owned(), "cfg".to_owned()];
    if let Some(target) = target {
        arguments.extend(["--target".to_owned(), target.to_owned()]);
    }
    if flags {
        arguments.extend(rustflags());
    }
    rustc(&arguments)
}

/// The flags cargo passes rustc: `CARGO_ENCODED_RUSTFLAGS`, else `RUSTFLAGS`.
fn rustflags() -> Vec<String> {
    match env::var("CARGO_ENCODED_RUSTFLAGS") {
        Ok(encoded) => encoded
            .split('\x1f')
            .filter(|flag| !flag.is_empty())
            .map(str::to_owned)
            .collect(),
        Err(_) => env::var("RUSTFLAGS")
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
    }
}

/// The host's target triple, as `rustc -vV` names it.
fn host() -> Result<String, String> {
    let version = rustc(&["-vV".to_owned()])?;
    version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "rustc -vV names no host".to_owned())
}

/// Runs rustc, the one `RUSTC` names or else the one on the path, as cargo does.
fn rustc(arguments: &[String]) -> Result<String, String> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(&rustc)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run {}: {error}", rustc.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} {} failed:\n{}",
            rustc.display(),
            arguments.join(" "),
            stderr.trim_end()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("{} printed no UTF-8: {error}", rustc.display()))
}

/// Finds the one flow that `path` selects from its crate root, resolved as the
/// compiler resolves it, in every target of every workspace member, or only of
/// `package`. A crate's search spans its own files and those of the
/// workspace's libraries, as a trait's default method may be in one. A file
/// that cannot be read or parsed is skipped and listed, as an error when the
/// flow is not found and as a warning when it is.
pub(super) fn find(
    path: &str,
    package: Option<&str>,
    options: &TargetOptions,
) -> Result<Found, String> {
    let crates = crates(package, options)?;
    let resolver = Resolver::new(&crates);
    let selected = |krate: &Crate| {
        !krate.dependency_only && package.is_none_or(|package| krate.package == package)
    };
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
            .filter(|(source, other)| {
                *source == index
                    || other.library
                        && (other.host == krate.host || other.host && !other.dependency_only)
            })
            .flat_map(|(source, _)| {
                files[source]
                    .iter()
                    .map(move |(at, declarations)| ((source, *at), declarations))
            })
            .collect();
        let mut ambiguous = false;
        let mut names = |question: Question<'_, (usize, usize)>| {
            answer(&crates, index, &resolver, question).unwrap_or_else(|| {
                ambiguous = true;
                false
            })
        };
        let selected = kaalang_compiler::select(flow_path, &listed, &mut names);
        if ambiguous {
            return Err(format!(
                "the flow path `{path}` is ambiguous through glob imports in {}",
                krate.label()
            ));
        }
        for (file, position) in selected {
            let ((source, at), declarations) = listed[file];
            let module = &crates[source].modules[at];
            // Crates read a shared file under their own `cfg`s, so its flow
            // is known by where it stands, not by its position among them.
            let at = declarations.flows[position]
                .function
                .sig
                .ident
                .span()
                .start();
            if seen.insert((module.file.canonical.clone(), at.line, at.column)) {
                let in_file = declarations.flows[position].path.clone();
                // Listed under the crate the path was resolved from, which `-p`
                // selects, and read for the crate whose file declares it.
                matches.push((index, source, module, in_file, position));
            }
        }
    }
    if matches.len() > 1 {
        let listed: Vec<(&Crate, String)> = matches
            .iter()
            .map(|(index, _, module, in_file, _)| {
                let at = format!("{}: {in_file}", module.file.path.display());
                (&crates[*index], at)
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
    let Some((_, source, module, _, position)) = matches.pop() else {
        let message = RenderError::FlowNotFound(path.to_owned()).to_string();
        return Err(with_skipped(message, &skipped));
    };
    warn_skipped(path, &skipped);
    Ok(Found {
        file: module.file.path.clone(),
        source: module.file.source.clone(),
        position,
        cfg: crates[source].cfg.clone(),
    })
}

/// Answers how names resolve for a flow path written from the root of crate
/// `krate`. A place names a crate and a file's own module in it.
fn answer(
    crates: &[Crate],
    krate: usize,
    resolver: &Resolver,
    question: Question<'_, (usize, usize)>,
) -> Option<bool> {
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
    let query = |def| (!matches!(def, Def::Ambiguous(_))).then_some(def);
    Some(match question {
        Question::Function { path, at, name } => {
            let (source, inside) = module(at);
            matches!(
                query(resolver.find_value(krate, 0, path))?,
                Def::Value(found, declared, value, "a function")
                    if (found, declared) == (source, inside) && name.unraw() == value
            )
        }
        Question::Type(path) => resolver.is_type(krate, 0, path)?,
        Question::Types {
            query,
            at,
            written,
            generics,
        } => resolver.types_match((krate, 0), query, module(at), written, generics),
        Question::Same(left, left_path, right, right_path) => {
            let last = |path: &syn::Path| path.segments.last().map(|segment| segment.ident.unraw());
            let (left_crate, left_module) = left.map_or((krate, 0), module);
            let (right_crate, right_module) = module(right);
            let left_def = resolver.find(left_crate, left_module, left_path);
            let left_def = if left.is_none() {
                query(left_def)?
            } else {
                left_def
            };
            match (
                left_def,
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
    })
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

pub(super) fn warn_skipped(path: &str, skipped: &BTreeSet<String>) {
    if !skipped.is_empty() {
        let warning = format!("warning: `{path}` may also be declared in a file that was skipped");
        eprintln!("{}: {}", crate::BIN_NAME, with_skipped(warning, skipped));
    }
}

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
    /// The cfg set the crate's files are read for.
    cfg: &'a kaalang_svg::Target,
    /// The files being read, outermost first; a `#[path]` loop stops here.
    open: Vec<PathBuf>,
    modules: Vec<Module>,
    types: Vec<TypeEntry>,
    skipped: Vec<String>,
}

impl<'a> Walk<'a> {
    fn new(cfg: &'a kaalang_svg::Target) -> Self {
        Self {
            cfg,
            open: Vec::new(),
            modules: Vec::new(),
            types: Vec::new(),
            skipped: Vec::new(),
        }
    }
}

/// Reads the file of the module at `path` and follows its `mod` declarations,
/// returning the module. A crate root, a `mod.rs`, and a `#[path]` file own
/// their directory; another file `a.rs` keeps its submodules in `a/`. A file
/// that cannot be read or parsed is skipped and listed, and its module stays
/// declared with an opaque scope, so that a path into it is unknown rather than
/// outside the workspace.
fn walk_file(
    walk: &mut Walk<'_>,
    file: &Path,
    owns_directory: bool,
    path: String,
    parent: Option<usize>,
) -> Option<usize> {
    let (canonical, source, parsed, scope) = match read(file) {
        Ok((canonical, source, mut parsed)) => {
            walk.cfg.configure(&mut parsed);
            (canonical, source, Some(parsed), Scope::default())
        }
        Err(error) => {
            // A missing module file may be one a build script generates.
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
fn walk_items(walk: &mut Walk<'_>, items: &[Item], module: usize, directory: &Path, base: &Path) {
    for item in items {
        let name = |ident: &syn::Ident| ident.unraw().to_string();
        let (bound, declared, vis) = match item {
            Item::Mod(declaration) => {
                let Some(child) = walk_module(walk, declaration, module, directory, base) else {
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
                        constructor_visibility(&walk.modules, module, declaration)
                {
                    walk.modules[module].scope.values.push((
                        name(ident),
                        "a struct constructor",
                        visible,
                    ));
                }
                let outer = &walk.modules[module];
                walk.types.push(TypeEntry {
                    module,
                    path: format!("{}{ident}", outer.path),
                    in_file: format!("{}{ident}", outer.in_file),
                    position: ident.span().start(),
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

/// Reads the module that `declaration` declares inside `parent`: an inline one,
/// or its file, its `#[path]` or default one. Configuring the source turned a
/// `cfg_attr` path that holds into a plain `#[path]`.
fn walk_module(
    walk: &mut Walk<'_>,
    declaration: &syn::ItemMod,
    parent: usize,
    directory: &Path,
    base: &Path,
) -> Option<usize> {
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
            walk_file(walk, &file, owns_directory, path, Some(parent))
        }
    }
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

/// The target's diagram group.
fn target_group(target: &Target) -> u8 {
    if target.is_test() {
        1
    } else if target.is_example() {
        2
    } else if target.is_bench() {
        3
    } else {
        0
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
    let cfg = kaalang_svg::Target::default();
    let mut walk = Walk::new(&cfg);
    walk.modules.push(Module {
        path: String::new(),
        parent: None,
        file: Rc::new(File {
            path: PathBuf::from("src/lib.rs"),
            canonical: PathBuf::from(format!("/{package}/src/lib.rs")),
            source: source.to_owned(),
            items: parsed.items.clone(),
        }),
        in_file: String::new(),
        scope: Scope::default(),
    });
    walk_items(
        &mut walk,
        &parsed.items,
        0,
        Path::new("src"),
        Path::new("src"),
    );
    Crate {
        package: package.to_owned(),
        package_id: package.to_owned(),
        name: package.replace('-', "_"),
        target: format!("lib `{package}`"),
        group: 0,
        library: true,
        host: false,
        dependency_only: false,
        root: PathBuf::from(format!("/{package}/src/lib.rs")),
        dependencies: Vec::new(),
        modules: walk.modules,
        types: walk.types,
        skipped: walk.skipped,
        cfg,
    }
}
