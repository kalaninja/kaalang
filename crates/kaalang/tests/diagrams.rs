//! Draws every executable fixture beside its source, so a change to the model or
//! the renderer shows up as a diagram diff instead of staying invisible.
//!
//! Geometry is reviewed in `git diff`. Every executable fixture must render;
//! a routing failure cannot silently delete a working diagram.

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn draws_a_diagram_beside_every_executable_fixture() {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut drawn = 0;
    for directory in fixture_directories(&tests) {
        // Collected up front because the loop below writes into this directory.
        let paths: Vec<PathBuf> = read_directory(&directory).collect();

        let mut current = Vec::new();
        for fixture in paths.iter().filter(|path| extension_is(path, "rs")) {
            let source = fs::read_to_string(fixture).unwrap();
            let names = kaalang_svg::flow_names(&source).unwrap();
            // A `mod.rs` that only lists the fixtures beside it draws nothing.
            if names.is_empty() {
                continue;
            }

            // A gallery mod.rs may group related flows and share their tests.
            // Other fixtures keep one flow named for the file or its folder.
            let stem = fixture.file_stem().unwrap();
            if stem != "mod" || directory.parent() != Some(tests.join("gallery").as_path()) {
                let named_after = if stem == "mod" {
                    directory.file_name().unwrap()
                } else {
                    stem
                };
                assert_eq!(
                    names,
                    [named_after.to_str().unwrap()],
                    "{}",
                    fixture.display()
                );
            }

            for flow in names {
                for (suffix, collapse_loops) in std::iter::once(("", false))
                    .chain(source.contains("#[cycle(").then_some(("_collapsed", true)))
                {
                    let svg = kaalang_svg::render_source_with_options(
                        &source,
                        &flow,
                        kaalang_svg::RenderOptions { collapse_loops },
                    )
                    .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
                    let diagram = directory.join(format!("{flow}{suffix}.svg"));
                    assert!(
                        !current.contains(&diagram),
                        "multiple fixtures draw {}",
                        diagram.display()
                    );
                    // Rewriting an unchanged diagram would spin file watchers on every run.
                    if !fs::read_to_string(&diagram).is_ok_and(|previous| previous == svg) {
                        fs::write(&diagram, svg).unwrap();
                    }
                    current.push(diagram);
                }
                drawn += 1;
            }
        }

        // A renamed or deleted fixture must not leave its old diagram behind.
        for path in paths.iter().filter(|path| extension_is(path, "svg")) {
            if !current.contains(path) {
                fs::remove_file(path).unwrap();
            }
        }
    }

    assert!(drawn > 0, "no executable fixture was found under {tests:?}");
}

/// Every module directory inside a test suite. `compile_fail` fixtures are
/// handed to trybuild rather than compiled as modules, so they have no `mod.rs`
/// and stay out of the gallery.
fn fixture_directories(tests: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = read_directory(tests)
        .filter(|suite| suite.is_dir())
        .flat_map(|suite| read_directory(&suite).collect::<Vec<_>>())
        .filter(|path| path.join("mod.rs").is_file())
        .collect();
    directories.sort();
    directories
}

fn read_directory(path: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()))
        .map(|entry| entry.expect("readable directory entry").path())
}

fn extension_is(path: &Path, extension: &str) -> bool {
    path.extension().is_some_and(|found| found == extension)
}

/// Every executable fixture is declared by the `mod.rs` beside it, and every one
/// that can finish carries a test of its own.
///
/// `render_source` reads the file rather than the module tree, so a fixture
/// renamed out of its module would go on producing a reviewed diagram that
/// nothing runs. A flow with no reachable end cannot be called at all — it
/// would not return — so those are exempt from the second half and from
/// nothing else.
#[test]
fn every_executable_fixture_is_declared_and_executed() {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    for directory in fixture_directories(&tests) {
        let declarations = fs::read_to_string(directory.join("mod.rs")).unwrap();
        for fixture in read_directory(&directory).filter(|path| extension_is(path, "rs")) {
            let stem = fixture.file_stem().unwrap().to_str().unwrap().to_owned();
            if stem == "mod" {
                continue;
            }
            assert!(
                declarations.contains(&format!("mod {stem};")),
                "{} is not declared by the mod.rs beside it",
                fixture.display()
            );
            let source = fs::read_to_string(&fixture).unwrap();
            if source.contains("#[test]") {
                continue;
            }
            let finishes = kaalang_svg::flow_names(&source)
                .unwrap()
                .iter()
                .any(|flow| {
                    kaalang_svg::render_source(&source, flow)
                        .unwrap()
                        .contains("End:")
                });
            assert!(
                !finishes,
                "{} declares a flow that finishes and no test that calls it",
                fixture.display()
            );
        }
    }
}
