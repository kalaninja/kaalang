//! Draws every behavior fixture beside its source, so a change to the model or
//! the renderer shows up as a diagram diff instead of staying invisible.
//!
//! Geometry is reviewed in `git diff`. Every behavior fixture must render;
//! a routing failure cannot silently delete a working diagram.

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn draws_a_diagram_beside_every_behavior_fixture() {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut drawn = 0;
    for directory in fixture_directories(&tests) {
        // Collected up front because the loop below writes into this directory.
        let paths: Vec<PathBuf> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();

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
                let svg = kaalang_svg::render_source(&source, &flow)
                    .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
                let diagram = directory.join(format!("{flow}.svg"));
                assert!(
                    !current.contains(&diagram),
                    "multiple fixtures draw {}",
                    diagram.display()
                );
                // Rewriting an unchanged diagram would spin file watchers on every run.
                if !fs::read_to_string(&diagram).is_ok_and(|previous| previous == svg) {
                    fs::write(&diagram, svg).unwrap();
                }
                drawn += 1;
                current.push(diagram);
            }
        }

        // A renamed or deleted fixture must not leave its old diagram behind.
        for path in paths.iter().filter(|path| extension_is(path, "svg")) {
            if !current.contains(path) {
                fs::remove_file(path).unwrap();
            }
        }
    }

    assert!(drawn > 0, "no behavior fixture was found under {tests:?}");
}

/// Every module directory inside a test suite. `compile_fail` fixtures are
/// handed to trybuild rather than compiled as modules, so they have no `mod.rs`
/// and stay out of the gallery.
fn fixture_directories(tests: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = read_directory(tests)
        .flat_map(|suite| read_directory(&suite).collect::<Vec<_>>())
        .filter(|path| path.join("mod.rs").is_file())
        .collect();
    directories.sort();
    directories
}

fn read_directory(path: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .map(|entry| entry.expect("readable directory entry").path())
}

fn extension_is(path: &Path, extension: &str) -> bool {
    path.extension().is_some_and(|found| found == extension)
}
