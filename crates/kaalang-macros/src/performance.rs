//! Measures the whole macro path: everything `#[kaalang]` pays to turn one
//! authored flow into Rust.
//!
//! `kaalang_model::build` now decides realizability, so the diagram
//! construction is part of every expansion even though the macro discards the
//! arrangement after lowering. The model's own measurements split that cost
//! from semantic analysis; this one bounds what it amounts to end to end,
//! beside the bindings and the emitted tokens that only this crate pays.

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

fn declares_a_flow(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("kaalang"))
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
            match item {
                syn::Item::Fn(function) if declares_a_flow(&function.attrs) => {
                    flows.push((function.sig.ident.to_string(), function));
                }
                syn::Item::Impl(block) => {
                    for item in block.items {
                        if let syn::ImplItem::Fn(method) = item
                            && declares_a_flow(&method.attrs)
                        {
                            let name = method.sig.ident.to_string();
                            flows.push((
                                name,
                                ItemFn {
                                    attrs: method.attrs,
                                    vis: method.vis,
                                    modifiers: method.modifiers,
                                    sig: method.sig,
                                    block: Box::new(method.block),
                                },
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// One pass of `expand` over every fixture flow.
fn pass(corpus: &[(String, ItemFn)]) -> Duration {
    let started = Instant::now();
    for (name, function) in corpus {
        let mut function = function.clone();
        super::expand(&mut function).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
    started.elapsed()
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
    let expansion = pass(&corpus);
    assert!(
        expansion < EXPANSION_BUDGET,
        "expanding {} flows took {expansion:?}, past the {EXPANSION_BUDGET:?} budget",
        corpus.len()
    );
}
