//! Loads executable fixture flows from `crates/kaalang/tests`, keeping the
//! performance corpus in sync with behavior tests, gallery examples, and stress fixtures.

use std::{
    fs,
    path::{Path, PathBuf},
};

use syn::ItemFn;

/// Fixture suites to load.
#[derive(Clone, Copy)]
pub enum Suite {
    /// Behavior tests and gallery examples outside the stress suite.
    Ordinary,
    /// Hand-written stress fixtures.
    Stress,
    /// Both ordinary and stress fixtures.
    All,
}

/// Selected fixture paths and source text in path order.
///
/// # Panics
///
/// Panics when the fixture tree is not where this crate expects it.
#[must_use]
pub fn files(suite: Suite) -> Vec<(PathBuf, String)> {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kaalang/tests")
        .canonicalize()
        .expect("the fixture tree exists");
    let mut files = Vec::new();
    collect(&tests, suite, &mut files);
    files.sort_unstable();
    files
}

/// Selected fixture flows in name order, with their stress-suite flag.
/// The `method` fixtures declare theirs in `impl` and `trait` blocks, which
/// [`kaalang_compiler::flows`] collects.
///
/// # Panics
///
/// Panics when a fixture stops parsing, as the renderer's corpus does.
#[must_use]
pub fn corpus(suite: Suite) -> Vec<(String, ItemFn, bool)> {
    let mut flows: Vec<(String, ItemFn, bool)> = files(suite)
        .iter()
        .flat_map(|(path, source)| {
            let file = syn::parse_file(source)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            kaalang_compiler::flows(&file.items)
                .into_iter()
                .map(|function| (function.sig.ident.to_string(), function, is_stress(path)))
        })
        .collect();
    flows.sort_by(|a, b| a.0.cmp(&b.0));
    flows
}

/// The flow named `name` in one fixture file's source.
///
/// # Panics
///
/// Panics when the source does not parse or declares no such flow.
#[must_use]
pub fn flow_named(source: &str, name: &str) -> ItemFn {
    let file = syn::parse_file(source).expect("the fixture parses");
    kaalang_compiler::flows(&file.items)
        .into_iter()
        .find(|function| function.sig.ident == name)
        .expect("the fixture declares the flow")
}

fn collect(directory: &Path, suite: Suite, files: &mut Vec<(PathBuf, String)>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", directory.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| {
                panic!(
                    "could not read an entry in {}: {error}",
                    directory.display()
                )
            })
            .path();
        if path.is_dir() {
            // A rejected program never builds, so it has nothing to measure.
            if path.file_name().is_some_and(|name| name == "compile_fail") {
                continue;
            }
            collect(&path, suite, files);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let included = match suite {
            Suite::Ordinary => !is_stress(&path),
            Suite::Stress => is_stress(&path),
            Suite::All => true,
        };
        if !included {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
        files.push((path, source));
    }
}

/// Checks representative flow and fixture kinds within the selected corpus.
///
/// # Panics
///
/// Panics when the selected corpus has lost a kind of flow or its stress tier.
pub fn assert_corpus_shape(flows: &[(String, ItemFn, bool)], suite: Suite) {
    if !matches!(suite, Suite::Stress) {
        for expected in [
            "destructure_singleton_tuple",
            "mutate_a_receiver",
            "doubled",
        ] {
            assert!(
                flows.iter().any(|(name, _, _)| name == expected),
                "the corpus lost {expected}"
            );
        }
    }
    if !matches!(suite, Suite::Ordinary) {
        assert!(
            flows.iter().any(|(_, _, stress)| *stress),
            "the fixture corpus lost its stress tier"
        );
    }
}

/// Whether a fixture belongs to the stress tier of the corpus.
#[must_use]
pub fn is_stress(path: &Path) -> bool {
    path.ancestors()
        .any(|directory| directory.ends_with("tests/stress"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selected_suites_partition_the_complete_corpus() {
        let ordinary = corpus(Suite::Ordinary);
        let stress = corpus(Suite::Stress);
        assert_corpus_shape(&ordinary, Suite::Ordinary);
        assert_corpus_shape(&stress, Suite::Stress);
        assert!(ordinary.iter().all(|(_, _, stress)| !*stress));
        assert!(stress.iter().all(|(_, _, stress)| *stress));

        let mut split: Vec<_> = ordinary
            .into_iter()
            .chain(stress)
            .map(|(name, _, stress)| (name, stress))
            .collect();
        let mut all: Vec<_> = corpus(Suite::All)
            .into_iter()
            .map(|(name, _, stress)| (name, stress))
            .collect();
        split.sort_unstable();
        all.sort_unstable();
        assert_eq!(split, all);
    }
}
