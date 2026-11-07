//! Finds a `#[kaalang]` flow in the current cargo workspace by its Rust path.

use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use cargo_metadata::{MetadataCommand, Target};
use kaalang_svg::RenderError;
use syn::{Attribute, Expr, ExprLit, Item, Lit, Meta, ext::IdentExt};

/// The file that declares a found flow, its source, and the flow's path within it.
pub(super) struct Found {
    pub(super) file: PathBuf,
    pub(super) source: String,
    pub(super) path_in_file: String,
}

/// Finds the one flow whose path from its crate root is `path`, searching every
/// target of every workspace member, or only of `package`. Only the modules
/// that the path passes through are read. A file that cannot be read or parsed
/// is skipped and listed, as an error when the flow is not found and as a
/// warning when it is.
pub(super) fn find(path: &str, package: Option<&str>) -> Result<Found, String> {
    let path = path.strip_prefix("crate::").unwrap_or(path);
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

    let mut matches = Vec::new();
    let mut skipped = BTreeSet::new();
    for package in packages {
        for target in package
            .targets
            .iter()
            .filter(|target| !target.is_custom_build())
        {
            let mut walk = Walk {
                wanted: path,
                open: Vec::new(),
                found: Vec::new(),
                skipped: Vec::new(),
            };
            walk_file(&mut walk, target.src_path.as_std_path(), true, "");
            skipped.extend(walk.skipped);
            for found in walk.found {
                matches.push((package.name.as_str(), target_label(target), found));
            }
        }
    }
    if matches.len() > 1 {
        let mut message = RenderError::AmbiguousFlow(path.to_owned()).to_string();
        message.push(':');
        for (package, target, found) in &matches {
            write!(
                message,
                "\n  {package}: {target} in {}",
                found.file.display()
            )
            .expect("writing to a String cannot fail");
        }
        // `-p` selects only in a package that declares the path once.
        let once = |package: &str| {
            matches
                .iter()
                .filter(|(other, ..)| *other == package)
                .count()
                == 1
        };
        if matches.iter().any(|(package, ..)| once(package)) {
            message.push_str("\nselect one with -p <PACKAGE>");
        }
        return Err(message);
    }
    let Some((.., found)) = matches.pop() else {
        let message = RenderError::FlowNotFound(path.to_owned()).to_string();
        return Err(with_skipped(message, &skipped));
    };
    if !skipped.is_empty() {
        let warning = format!("warning: `{path}` may also be declared in a file that was skipped");
        eprintln!("{}: {}", crate::BIN_NAME, with_skipped(warning, &skipped));
    }
    Ok(found)
}

/// Appends the files the walk could not search to `message`.
fn with_skipped(mut message: String, skipped: &BTreeSet<String>) -> String {
    if !skipped.is_empty() {
        message.push_str("\nthese files and their submodules could not be searched:");
        for file in skipped {
            write!(message, "\n  {file}").expect("writing to a String cannot fail");
        }
    }
    message
}

/// One crate's module tree, searched for the flows at `wanted`.
struct Walk<'a> {
    wanted: &'a str,
    /// The files being read, outermost first; a `#[path]` loop stops here.
    open: Vec<PathBuf>,
    found: Vec<Found>,
    /// Each file that could not be read or parsed, with the reason.
    skipped: Vec<String>,
}

/// Reads the file of `module` and follows its `mod` declarations. A crate root,
/// a `mod.rs`, and a `#[path]` file own their directory; another file `a.rs`
/// keeps its submodules in `a/`.
fn walk_file(walk: &mut Walk, file: &Path, owns_directory: bool, module: &str) {
    let skip = |walk: &mut Walk, error: &dyn std::fmt::Display| {
        walk.skipped.push(format!("{}: {error}", file.display()));
    };
    let canonical = match fs::canonicalize(file) {
        Ok(canonical) => canonical,
        // A module file may be generated or belong to another configuration.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(error) => return skip(walk, &error),
    };
    if walk.open.contains(&canonical) {
        return;
    }
    let source = match fs::read_to_string(file) {
        Ok(source) => source,
        Err(error) => return skip(walk, &error),
    };
    let parsed = match syn::parse_file(&source) {
        Ok(parsed) => parsed,
        Err(error) => return skip(walk, &error),
    };
    // Both `#[kaalang]` and `#[kaalang::kaalang]` contain the word, so most
    // files skip the second parse.
    if source.contains("kaalang") {
        for path_in_file in kaalang_svg::flow_names(&source).expect("the source parsed above") {
            if walk.wanted.strip_prefix(module) == Some(path_in_file.as_str()) {
                walk.found.push(Found {
                    file: file.to_owned(),
                    source: source.clone(),
                    path_in_file,
                });
            }
        }
    }

    let parent = file.parent().unwrap_or(Path::new(""));
    let directory = match file.file_stem() {
        Some(stem) if !owns_directory => parent.join(stem),
        _ => parent.to_owned(),
    };
    walk.open.push(canonical);
    walk_items(walk, &parsed.items, module, &directory, parent);
    walk.open.pop();
}

/// Follows the `mod` declarations among `items`. `directory` holds their
/// default files, and a `#[path]` is relative to `base`.
fn walk_items(walk: &mut Walk, items: &[Item], module: &str, directory: &Path, base: &Path) {
    for item in items {
        let Item::Mod(declaration) = item else {
            continue;
        };
        let module = format!("{module}{}::", declaration.ident);
        // Every flow inside the module has a path that starts with the module's.
        if !walk.wanted.starts_with(&module) {
            continue;
        }
        let name = declaration.ident.unraw().to_string();
        let path = path_attribute(&declaration.attrs);
        match (&declaration.content, path) {
            (Some((_, items)), path) => {
                // An inline module's `#[path]` names its directory.
                let directory = path.map_or_else(|| directory.join(name), |path| base.join(path));
                walk_items(walk, items, &module, &directory, &directory);
            }
            (None, Some(path)) => walk_file(walk, &base.join(path), true, &module),
            (None, None) => {
                let file = directory.join(format!("{name}.rs"));
                if file.is_file() {
                    walk_file(walk, &file, false, &module);
                } else {
                    walk_file(walk, &directory.join(name).join("mod.rs"), true, &module);
                }
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
