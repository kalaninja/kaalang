//! Reads the current cargo workspace: each crate's module tree from its root,
//! with the names every module declares and imports, and its type
//! declarations in source order.

use std::{
    collections::BTreeSet,
    env,
    fmt::Write as _,
    fs, io, iter,
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

use cargo_metadata::{CargoOpt, DependencyKind, MetadataCommand, Target};
use kaalang_compiler::{Declarations, FlowPath, Place, Question};
use kaalang_workspace::{
    Files,
    names::{Crate as CompiledCrate, Def, ModuleTree, Resolver},
};
use syn::ext::IdentExt;

/// The file that declares a found flow, its source, the flow's position among
/// the file's flows, and the cfg set its crate is read for.
pub(super) struct Found<'a> {
    pub(super) file: &'a Path,
    pub(super) source: &'a str,
    pub(super) position: usize,
    pub(super) cfg: &'a kaalang_compiler::CfgSet,
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
    /// The target as errors list it: `` lib `app` ``.
    pub(super) target: String,
    /// Where it stands among its package's crates: its library and binaries
    /// 0, then its tests 1, examples 2, and benches 3.
    pub(super) group: u8,
    /// An additional host reading of a library, reached through dependencies only.
    pub(super) dependency_only: bool,
    pub(super) root: PathBuf,
    /// Existing source paths, including unreadable files and both sides of an ambiguous module.
    pub(super) sources: BTreeSet<PathBuf>,
    /// Each file that could not be read or parsed, with the reason.
    pub(super) skipped: Vec<String>,
    pub(super) names: CompiledCrate,
}

impl Deref for Crate {
    type Target = CompiledCrate;

    fn deref(&self) -> &CompiledCrate {
        &self.names
    }
}

impl Crate {
    /// The crate as an error lists it.
    pub(super) fn label(&self) -> String {
        format!("{}: {}", self.package, self.target)
    }
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
            let root = target.src_path.clone().into_std_path_buf();
            let mut sources = Sources::default();
            let tree = ModuleTree::read(root.clone(), &cfg, &mut sources);
            let group = target_group(target);
            let names = CompiledCrate {
                package_id: package.id.to_string(),
                name: target.name.replace('-', "_"),
                library: group == 0 && !target.is_bin(),
                host: for_host,
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
                modules: tree.modules,
                types: tree.types,
                cfg,
            };
            crates.push(Crate {
                package: package.name.to_string(),
                target: target_label(target),
                group,
                dependency_only,
                root,
                sources: sources.0,
                skipped: tree.skipped,
                names,
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
) -> Result<kaalang_compiler::CfgSet, String> {
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
pub(super) fn find<'a>(
    crates: &'a [Crate],
    path: &str,
    package: Option<&str>,
) -> Result<Found<'a>, String> {
    let resolver = Resolver::new(crates.iter().map(|krate| &krate.names));
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
            answer(crates, index, &resolver, question).unwrap_or_else(|| {
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
            &format!("multiple `#[kaa]` functions have the path `{path}`"),
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
        let message = format!("`#[kaa]` function `{path}` was not found");
        return Err(with_skipped(message, &skipped));
    };
    warn_skipped(path, &skipped);
    Ok(Found {
        file: &module.file.path,
        source: &module.file.source,
        position,
        cfg: &crates[source].cfg,
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
                (Def::Type(left), Def::Type(right)) => left == right,
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
pub(super) fn ambiguous(heading: &str, matches: &[(&Crate, String)]) -> String {
    let mut message = heading.to_owned();
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

/// The file system, recording each existing source an export must protect,
/// even one that cannot be read.
#[derive(Default)]
struct Sources(BTreeSet<PathBuf>);

impl Files for Sources {
    fn canonical(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        match fs::canonicalize(path) {
            Ok(canonical) => {
                self.0.insert(canonical.clone());
                Ok(Some(canonical))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn read(&mut self, canonical: &Path) -> io::Result<String> {
        fs::read_to_string(canonical)
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
