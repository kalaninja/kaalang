//! Draws every behavior fixture beside its source, so a change to the model or
//! the renderer shows up as a diagram diff instead of staying invisible.
//!
//! The diagrams are generated, not asserted: `git diff` is the review surface.

use std::{fs, path::PathBuf};

#[test]
fn draws_a_diagram_beside_every_behavior_fixture() {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut drawn = 0;
    for entry in fs::read_dir(&tests).unwrap() {
        let directory = entry.unwrap().path();
        let fixture = directory.join("behavior.rs");
        if !fixture.is_file() {
            continue;
        }

        let source = fs::read_to_string(&fixture).unwrap();
        let mut current = Vec::new();
        for flow in contour_svg::flow_names(&source).unwrap() {
            let svg = contour_svg::render_source(&source, &flow)
                .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
            let diagram = directory.join(format!("{flow}.svg"));
            // Rewriting an unchanged diagram would spin file watchers on every run.
            if !fs::read_to_string(&diagram).is_ok_and(|previous| previous == svg) {
                fs::write(&diagram, svg).unwrap();
            }
            drawn += 1;
            current.push(diagram);
        }

        // A renamed or deleted flow must not leave its old diagram behind.
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|extension| extension == "svg")
                && !current.contains(&path)
            {
                fs::remove_file(path).unwrap();
            }
        }
    }

    assert!(drawn > 0, "no behavior fixture was found under {tests:?}");
}
