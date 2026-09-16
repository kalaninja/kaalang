//! Loads behavior and gallery flows from `crates/kaalang/tests`, keeping the
//! performance corpus in sync with executable examples.

use std::fs;
use std::path::{Path, PathBuf};

use syn::ItemFn;

/// Fixture paths and source text in path order.
///
/// # Panics
///
/// Panics when the fixture tree is not where this crate expects it.
#[must_use]
pub fn files() -> Vec<(PathBuf, String)> {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kaalang/tests")
        .canonicalize()
        .expect("the fixture tree exists");
    let mut files = Vec::new();
    collect(&tests, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Every flow of every fixture, ordered by name. The `method` fixtures declare
/// theirs in `impl` and `trait` blocks, which [`kaalang_compiler::flows`] collects.
///
/// # Panics
///
/// Panics when a fixture stops parsing, as the renderer's corpus does.
#[must_use]
pub fn corpus() -> Vec<(String, ItemFn)> {
    let mut flows: Vec<(String, ItemFn)> = files()
        .iter()
        .flat_map(|(path, source)| {
            let file = syn::parse_file(source)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            kaalang_compiler::flows(&file.items)
        })
        .map(|function| (function.sig.ident.to_string(), function))
        .collect();
    flows.sort_by(|a, b| a.0.cmp(&b.0));
    flows
}

fn collect(directory: &Path, files: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // A rejected program never builds, so it has nothing to measure.
            if path.file_name().is_some_and(|name| name == "compile_fail") {
                continue;
            }
            collect(&path, files);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        if let Ok(source) = fs::read_to_string(&path) {
            files.push((path, source));
        }
    }
}

/// Asserts the corpus is the whole tree, naming a free flow, one in an `impl`
/// and one in a `trait`, so a loader that stops seeing a kind fails here.
///
/// # Panics
///
/// Panics when the corpus is short or has lost a kind of flow.
pub fn assert_whole_tree(flows: &[(String, ItemFn)]) {
    assert!(
        flows.len() > 100,
        "the corpus should be the whole tree, found {}",
        flows.len()
    );
    for expected in [
        "destructure_singleton_tuple",
        "mutate_a_receiver",
        "doubled",
    ] {
        assert!(
            flows.iter().any(|(name, _)| name == expected),
            "the corpus lost {expected}"
        );
    }
}
