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
    for entry in fs::read_dir(&tests).unwrap() {
        let directory = entry.unwrap().path().join("behavior");
        if !directory.is_dir() {
            continue;
        }

        // Collected up front because the loop below writes into this directory.
        let paths: Vec<PathBuf> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();

        let mut current = Vec::new();
        for fixture in paths.iter().filter(|path| extension_is(path, "rs")) {
            let flow = fixture.file_stem().unwrap().to_str().unwrap();
            if flow == "mod" {
                continue;
            }

            let source = fs::read_to_string(fixture).unwrap();
            // One flow per file, named for it, is what keeps a diagram beside
            // the source it was drawn from.
            assert_eq!(
                kaalang_svg::flow_names(&source).unwrap(),
                [flow],
                "{}",
                fixture.display()
            );

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

fn extension_is(path: &Path, extension: &str) -> bool {
    path.extension().is_some_and(|found| found == extension)
}
