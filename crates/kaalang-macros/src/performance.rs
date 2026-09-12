//! Measures the whole macro path: everything `#[kaalang]` pays to turn one
//! authored flow into Rust.
//!
//! `kaalang_model::build` now decides realizability, so the diagram
//! construction is part of every expansion even though the macro discards the
//! arrangement after lowering. The model's own measurements split that cost
//! from semantic analysis; this one shows what it amounts to end to end,
//! beside the bindings and the emitted tokens that only this crate pays.
//!
//! `measure_the_expansion_cost` prints the numbers and is ignored by default,
//! because a wall-clock reading is not a stable assertion. The budget test
//! beside it asserts a bound this crate records rather than one the plan
//! published, since the plan asks for the measurement and sets no target.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use syn::ItemFn;

/// Every flow of every behavior and gallery fixture.
fn corpus() -> Vec<(String, ItemFn)> {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kaalang/tests")
        .canonicalize()
        .expect("the fixture tree exists");
    let mut flows = Vec::new();
    collect(&tests, &mut flows);
    flows
}

fn collect(directory: &Path, flows: &mut Vec<(String, ItemFn)>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "compile_fail") {
                continue;
            }
            collect(&path, flows);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(file) = syn::parse_file(&source) else {
            continue;
        };
        for item in file.items {
            if let syn::Item::Fn(function) = item
                && function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("kaalang"))
            {
                flows.push((function.sig.ident.to_string(), function));
            }
        }
    }
}

/// One pass of `expand` over every fixture flow, and one of `build` alone for
/// comparison.
fn pass(corpus: &[(String, ItemFn)]) -> (Duration, Duration) {
    let started = Instant::now();
    for (name, function) in corpus {
        let mut function = function.clone();
        super::expand(&mut function).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
    let expansion = started.elapsed();
    let started = Instant::now();
    for (name, function) in corpus {
        kaalang_model::build(function).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
    (expansion, started.elapsed())
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

/// One warm-up pass and twenty timed passes, reporting the corpus totals and
/// the share the model's own work takes of them.
#[test]
#[ignore = "a wall-clock measurement, not an assertion; run it with --ignored"]
fn measure_the_expansion_cost() {
    let corpus = corpus();
    let _ = pass(&corpus);
    let mut expansion = Vec::new();
    let mut model = Vec::new();
    for _ in 0..20 {
        let (whole, built) = pass(&corpus);
        expansion.push(whole);
        model.push(built);
    }
    println!("corpus flows: {}", corpus.len());
    println!("expansion, corpus total median: {:?}", median(expansion));
    println!("build alone, corpus total median: {:?}", median(model));
}

/// What one pass of the whole macro path over the corpus is allowed to cost.
///
/// The plan asks for this measurement, and records this bound with it rather
/// than quoting a published one: one second against a measured median of about
/// 280 ms over 142 flows, which is loose enough for a loaded machine and tight
/// enough to catch the kind of scaling regression the model's own gates found.
const EXPANSION_BUDGET: Duration = Duration::from_secs(1);

/// Expanding every fixture flow, bindings and emitted Rust included, stays
/// inside that bound.
#[test]
fn expansion_stays_inside_its_budget() {
    let corpus = corpus();
    assert!(corpus.len() > 100, "the corpus should be the whole tree");
    // One warm-up pass, so the first run's page faults are not measured.
    let _ = pass(&corpus);
    let (expansion, _) = pass(&corpus);
    assert!(
        expansion < EXPANSION_BUDGET,
        "expanding {} flows took {expansion:?}, past the {EXPANSION_BUDGET:?} budget",
        corpus.len()
    );
}
