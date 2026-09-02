//! Draws every behavior fixture beside its source, so a change to the model or
//! the renderer shows up as a diagram diff instead of staying invisible.
//!
//! The diagrams are generated, not asserted: `git diff` is the review surface.

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

            // One flow per file, named for the file — or for the folder, when
            // the fixture is the folder — is what keeps a diagram beside the
            // source it was drawn from.
            let stem = fixture.file_stem().unwrap();
            let named_after = if stem == "mod" {
                directory.file_name().unwrap()
            } else {
                stem
            };
            let flow = named_after.to_str().unwrap();
            assert_eq!(names, [flow], "{}", fixture.display());

            let svg = kaalang_svg::render_source(&source, flow)
                .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
            let diagram = directory.join(format!("{flow}.svg"));
            // Rewriting an unchanged diagram would spin file watchers on every run.
            if !fs::read_to_string(&diagram).is_ok_and(|previous| previous == svg) {
                fs::write(&diagram, svg).unwrap();
            }
            drawn += 1;
            current.push(diagram);
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
