//! Reads the current cargo workspace: each crate's module tree from its root,
//! with the names every module declares and imports, and its type
//! declarations in source order.

use std::{
    collections::BTreeSet,
    env,
    fmt::Write as _,
    fs, io,
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
};

use cargo_metadata::{CargoOpt, DependencyKind, MetadataCommand, Target};
use kaalang_workspace::{
    Files, crate_name,
    names::{Crate as CompiledCrate, FlowPath, FoundFlow, ModuleTree, Resolver},
    target_group,
};

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
    /// The target as errors list it: `` lib `app` ``.
    pub(super) target: String,
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
/// `package`, when given, is found among them, each for the target `options`
/// name: a procedural macro too, though cargo builds it for the host.
pub(super) fn crates(package: Option<&str>, options: &TargetOptions) -> Result<Vec<Crate>, String> {
    let printed = rustc_cfg(options.target.as_deref())?;
    let metadata = metadata(package, options)?;
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
        // A build script draws no cards.
        let targets = package.targets.iter().filter_map(|target| {
            let kinds: Vec<String> = target.kind.iter().map(ToString::to_string).collect();
            Some((target, target_group(&kinds)?))
        });
        for (target, group) in targets {
            let resolved = metadata
                .resolve
                .as_ref()
                .and_then(|resolve| resolve.nodes.iter().find(|node| node.id == package.id));
            // Cargo builds no target whose required features are off.
            if !required_features(&target.required_features, package, resolved, &metadata) {
                continue;
            }
            let features: Vec<String> = resolved
                .map(|node| node.features.iter().map(ToString::to_string).collect())
                .unwrap_or_default();
            let cfg = cfg_set(target, &printed, &features)?;
            let root = target.src_path.clone().into_std_path_buf();
            let mut sources = Sources::default();
            let tree = ModuleTree::read(root.clone(), &cfg, &mut sources);
            let names = CompiledCrate {
                package_id: package.id.to_string(),
                package: package.name.to_string(),
                group,
                name: crate_name(&target.name),
                root,
                library: group == 0 && !target.is_bin(),
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
                target: target_label(target),
                sources: sources.0,
                skipped: tree.skipped,
                names,
            });
        }
    }
    Ok(crates)
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
                                    && crate_name(&target.name) == edge.name
                            })
                        },
                        |rename| crate_name(rename) == edge.name,
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

/// The workspace as `cargo metadata` describes it for the target `options`
/// name, or the host, with the features they select. As with
/// `cargo build -p`, a feature without a package is `package`'s.
fn metadata(
    package: Option<&str>,
    options: &TargetOptions,
) -> Result<cargo_metadata::Metadata, String> {
    let triple = match &options.target {
        Some(triple) => triple.clone(),
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
/// cargo passes.
fn rustc_cfg(target: Option<&str>) -> Result<String, String> {
    let mut arguments = vec!["--print".to_owned(), "cfg".to_owned()];
    if let Some(target) = target {
        arguments.extend(["--target".to_owned(), target.to_owned()]);
    }
    arguments.extend(rustflags());
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

/// The crates in the card order of RFC 0009 §4.3, `first` being the focus
/// type's.
pub(super) fn order(crates: &[Crate], first: usize) -> Vec<usize> {
    let focus = crates.get(first).map(|krate| krate.target());
    let mut order: Vec<usize> = (0..crates.len()).collect();
    order.sort_by_key(|&index| crates[index].target().order(focus));
    order
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
    let selected = |krate: &Crate| package.is_none_or(|package| krate.package == package);
    let roots: Vec<(usize, Vec<usize>)> = order(crates, usize::MAX)
        .into_iter()
        .filter(|&index| selected(&crates[index]))
        .map(|index| {
            let spanned =
                (0..crates.len()).filter(|&source| source == index || crates[source].library);
            (index, spanned.collect())
        })
        .collect();
    let mut matches = match FlowPath::parse(path) {
        Some(flow_path) => resolver.select_flows(&flow_path, &roots).map_err(|root| {
            format!(
                "the flow path `{path}` is ambiguous through glob imports in {}",
                crates[root].label()
            )
        })?,
        None => Vec::new(),
    };
    let file = |found: &FoundFlow| &crates[found.krate].modules[found.module].file;
    if matches.len() > 1 {
        // Each listed under the crate the path was resolved from, which `-p`
        // selects.
        let listed: Vec<(&Crate, String)> = matches
            .iter()
            .map(|found| {
                let at = format!("{}: {}", file(found).path.display(), found.in_file);
                (&crates[found.root], at)
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
    let Some(found) = matches.pop() else {
        let message = format!("`#[kaa]` function `{path}` was not found");
        return Err(with_skipped(message, &skipped));
    };
    warn_skipped(path, &skipped);
    // Read for the crate whose file declares it.
    let declaring = file(&found);
    Ok(Found {
        file: &declaring.path,
        source: &declaring.source,
        position: found.position,
        cfg: &crates[found.krate].cfg,
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
            // A path through a regular file names nothing, as for rustc.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn read(&mut self, canonical: &Path) -> io::Result<String> {
        fs::read_to_string(canonical)
    }
}

/// Names a crate by its target, as `` lib `app` `` or `` test `cli` ``.
fn target_label(target: &Target) -> String {
    let kinds: Vec<String> = target.kind.iter().map(ToString::to_string).collect();
    format!("{} `{}`", kinds.join(", "), target.name)
}
