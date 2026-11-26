//! Draws test fixtures beside their source, so a change to the model or
//! the renderer shows up as a diagram diff.
//!
//! Every executable fixture must render; a routing failure cannot silently
//! delete a working diagram. Fixtures must be declared by their module and
//! carry a test unless their flows never finish.

use std::{
    fs,
    path::{Path, PathBuf},
};

use kaalang_compiler::{SourceFile, SourceType};
use kaalang_testing::corpus::TARGET;

#[test]
#[ignore = "run just rust svg to regenerate fixture diagrams"]
fn draws_a_diagram_beside_every_executable_fixture() {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut drawn = 0;
    for directory in fixture_directories(&tests) {
        let declarations = fs::read_to_string(directory.join("mod.rs")).unwrap();
        let module = directory.file_name().unwrap().to_str().unwrap();
        let suite = fs::read_to_string(directory.parent().unwrap().join("main.rs")).unwrap();
        assert!(
            suite.contains(&format!("mod {module};")),
            "{} is not declared by its suite's main.rs",
            directory.display()
        );
        // Collected up front because the loop below writes into this directory.
        let paths: Vec<PathBuf> = read_directory(&directory).collect();

        let mut current = Vec::new();
        for fixture in paths.iter().filter(|path| extension_is(path, "rs")) {
            let stem = fixture.file_stem().unwrap().to_str().unwrap();
            if stem != "mod" {
                assert!(
                    declarations.contains(&format!("mod {stem};")),
                    "{} is not declared by the mod.rs beside it",
                    fixture.display()
                );
            }
            let source = fs::read_to_string(fixture).unwrap();
            let file = SourceFile::parse(&source, &TARGET)
                .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
            let flows = kaalang_compiler::flows(file.items());
            // A method's diagram is named after the method, without its impl.
            let names: Vec<&str> = flows
                .iter()
                .map(|flow| {
                    let name = flow.path.rsplit("::").next().unwrap();
                    name.strip_prefix("r#").unwrap_or(name)
                })
                .collect();
            // A file without flows draws its types, and a `mod.rs` that
            // only lists the fixtures beside it draws nothing.
            if names.is_empty() {
                for (name, svg) in type_diagrams(&file, fixture, module) {
                    draw(&mut current, directory.join(format!("{name}.svg")), &svg);
                    drawn += 1;
                }
                continue;
            }

            let expected = if stem == "mod" { module } else { stem };
            assert_eq!(names, [expected], "{}", fixture.display());

            for (name, svg) in flow_diagrams(&source, &file, fixture, stem, &names) {
                draw(&mut current, directory.join(format!("{name}.svg")), &svg);
            }
            drawn += names.len();
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

/// Each flow's diagram as the fixture target compiles it, named after the
/// flow, then its collapsed one when that differs.
fn flow_diagrams(
    source: &str,
    file: &SourceFile<'_>,
    fixture: &Path,
    stem: &str,
    names: &[&str],
) -> Vec<(String, String)> {
    let mut diagrams: Vec<(String, String)> = Vec::new();
    for (position, name) in names.iter().enumerate() {
        for collapse_cycles in kaalang_testing::corpus::views(source) {
            let suffix = if collapse_cycles { "_collapsed" } else { "" };
            let (svg, _) = file
                .flow(position, collapse_cycles)
                .map_err(|error| error.to_string())
                .and_then(|flow| kaalang_svg::render_flow(flow).map_err(|error| error.to_string()))
                .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
            if !collapse_cycles && stem != "mod" {
                assert!(
                    source.contains("#[test]") || !svg.contains("End:"),
                    "{} declares a flow that finishes and no test that calls it",
                    fixture.display()
                );
            }
            // The expanded diagram was drawn just before.
            if collapse_cycles
                && diagrams
                    .last()
                    .is_some_and(|(_, expanded)| *expanded == svg)
            {
                continue;
            }
            diagrams.push((format!("{name}{suffix}"), svg));
        }
    }
    diagrams
}

fn draw(current: &mut Vec<PathBuf>, diagram: PathBuf, svg: &str) {
    assert!(
        !current.contains(&diagram),
        "multiple fixtures draw {}",
        diagram.display()
    );
    // Rewriting an unchanged diagram would spin file watchers on every run.
    if !fs::read_to_string(&diagram).is_ok_and(|previous| previous == svg) {
        fs::write(&diagram, svg)
            .unwrap_or_else(|error| panic!("could not write {}: {error}", diagram.display()));
    }
    current.push(diagram);
}

/// One diagram for every type of a fixture without flows, as the fixture
/// target compiles it. Incoming types are matched by the bare name a `refer`
/// writes, standing in for the resolver of `cargo kaalang`.
fn type_diagrams(file: &SourceFile<'_>, fixture: &Path, case: &str) -> Vec<(String, String)> {
    let fixture = fixture.display();
    let types: Vec<SourceType> = kaalang_compiler::type_items(file.items())
        .into_iter()
        .map(|(path, _)| {
            file.ty(&path)
                .unwrap_or_else(|error| panic!("{fixture}: {error}"))
        })
        .collect();
    types
        .iter()
        .enumerate()
        .map(|(index, focus)| {
            let name = last_segment(&focus.path);
            let drawn: Vec<(&SourceType, Vec<usize>)> = types
                .iter()
                .map(|ty| (ty, kaalang_testing::corpus::naming(&ty.declaration, name)))
                .collect();
            let incoming: Vec<_> = drawn
                .iter()
                .filter(|(ty, positions)| ty.path != focus.path && !positions.is_empty())
                .map(card)
                .collect();
            let own = drawn
                .iter()
                .find(|(ty, _)| ty.path == focus.path)
                .map(card)
                .expect("the focus is a listed type");
            let groups = vec![(); incoming.len()];
            let options = kaalang_svg::TypeOptions::default();
            let (svg, _) = kaalang_svg::render_type(&own, &incoming, &groups, options)
                .unwrap_or_else(|error| panic!("{fixture}: {}: {error}", focus.path));
            let file = if index == 0 {
                case.to_owned()
            } else {
                snake_case(name)
            };
            (file, svg)
        })
        .collect()
}

fn snake_case(name: &str) -> String {
    let mut snake = String::new();
    for (index, character) in name.char_indices() {
        if character.is_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.extend(character.to_lowercase());
    }
    snake
}

fn last_segment(path: &str) -> &str {
    path.rsplit_once("::").map_or(path, |(_, name)| name)
}

fn card<'a>((ty, relationships): &'a (&'a SourceType, Vec<usize>)) -> kaalang_svg::TypeCard<'a> {
    kaalang_svg::TypeCard {
        ty,
        header: &ty.path,
        relationships,
    }
}

/// Every module directory inside a test suite. `compile_fail` fixtures are
/// handed to trybuild rather than compiled as modules, so they have no `mod.rs`
/// and stay out of this list.
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
