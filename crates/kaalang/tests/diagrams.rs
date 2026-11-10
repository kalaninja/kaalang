//! Draws every executable fixture beside its source, so a change to the model or
//! the renderer shows up as a diagram diff instead of staying invisible.
//!
//! Geometry is reviewed in `git diff`. Every executable fixture must render;
//! a routing failure cannot silently delete a working diagram.
//! Fixtures must be declared by their module and carry a test unless their
//! flows never finish.

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
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
            let flows = kaalang_svg::flow_names(&source).unwrap();
            // A method's diagram is named after the method, without its impl.
            let names: Vec<&str> = flows
                .iter()
                .map(|path| {
                    let name = path.rsplit("::").next().unwrap();
                    name.strip_prefix("r#").unwrap_or(name)
                })
                .collect();
            // A file without flows draws its types, and a `mod.rs` that
            // only lists the fixtures beside it draws nothing.
            if names.is_empty() {
                for (name, svg) in type_diagrams(&source, fixture, module) {
                    draw(&mut current, directory.join(format!("{name}.svg")), &svg);
                    drawn += 1;
                }
                continue;
            }

            // Gallery files may group related flows, starting with the file's
            // namesake. Other fixtures keep exactly one flow with that name.
            let gallery = directory.parent() == Some(tests.join("gallery").as_path());
            if stem != "mod" || !gallery {
                let expected = if stem == "mod" {
                    directory.file_name().unwrap().to_str().unwrap()
                } else {
                    stem
                };
                if gallery {
                    assert_eq!(
                        names.first().copied(),
                        Some(expected),
                        "{}",
                        fixture.display()
                    );
                } else {
                    assert_eq!(names, [expected], "{}", fixture.display());
                }
            }

            for (flow, name) in flows.iter().zip(names) {
                let mut expanded = None;
                for (suffix, collapse_cycles) in std::iter::once(("", false)).chain(
                    source
                        .contains("#[kaa::cycle(")
                        .then_some(("_collapsed", true)),
                ) {
                    let svg = kaalang_svg::render_source_with_options(
                        &source,
                        flow,
                        kaalang_svg::RenderOptions { collapse_cycles },
                    )
                    .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
                    if !collapse_cycles && stem != "mod" {
                        assert!(
                            source.contains("#[test]") || !svg.contains("End:"),
                            "{} declares a flow that finishes and no test that calls it",
                            fixture.display()
                        );
                    }
                    if collapse_cycles && expanded.as_ref() == Some(&svg) {
                        continue;
                    }
                    draw(
                        &mut current,
                        directory.join(format!("{name}{suffix}.svg")),
                        &svg,
                    );
                    if !collapse_cycles {
                        expanded = Some(svg);
                    }
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

/// One diagram for every type path of a fixture without flows, with each of
/// its declarations, named after the type, except the first: the subject of
/// the `case` is named after it. A type's incoming types are those in the same
/// file whose `refer` names it bare, a convention that stands in for the
/// resolver of `cargo kaalang`.
fn type_diagrams(source: &str, fixture: &Path, case: &str) -> Vec<(String, String)> {
    let fixture = fixture.display();
    let file = syn::parse_file(source).unwrap_or_else(|error| panic!("{fixture}: {error}"));
    // Each declaration with its place among those of its path, its `cfg`s,
    // and the targets of its relationships, in source order.
    let mut types: Vec<(String, usize, Vec<String>, Vec<String>)> = Vec::new();
    for (path, item) in kaalang_compiler::type_items(&file.items) {
        let declaration = kaalang_compiler::type_declaration(&item)
            .unwrap_or_else(|error| panic!("{fixture}: {path}: {error}"));
        let targets = declaration
            .relationships()
            .iter()
            .map(|relationship| {
                relationship
                    .target
                    .get_ident()
                    .map_or_else(String::new, ToString::to_string)
            })
            .collect();
        let occurrence = types.iter().filter(|(other, ..)| *other == path).count();
        types.push((path, occurrence, cfgs(&item), targets));
    }
    types
        .iter()
        .filter(|(_, occurrence, ..)| *occurrence == 0)
        .enumerate()
        .map(|(index, (focus, ..))| {
            let name = last_segment(focus);
            let drawn: Vec<(&str, usize, &[String], Vec<usize>)> = types
                .iter()
                .map(|(path, occurrence, cfgs, targets)| {
                    let positions = (0..targets.len())
                        .filter(|&index| targets[index] == name)
                        .collect();
                    // Only the declarations of a path declared more than once
                    // need their `cfg`s to tell them apart.
                    let declared_once =
                        types.iter().filter(|(other, ..)| other == path).count() == 1;
                    let cfgs: &[String] = if declared_once { &[] } else { cfgs };
                    (path.as_str(), *occurrence, cfgs, positions)
                })
                .collect();
            let incoming: Vec<_> = drawn
                .iter()
                .filter(|(path, .., positions)| path != focus && !positions.is_empty())
                .map(|drawn| card(source, drawn))
                .collect();
            let own: Vec<_> = drawn
                .iter()
                .filter(|(path, ..)| path == focus)
                .map(|drawn| card(source, drawn))
                .collect();
            let svg =
                kaalang_svg::render_type(&own, &[&incoming], kaalang_svg::TypeOptions::default())
                    .unwrap_or_else(|error| panic!("{fixture}: {focus}: {error}"));
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

fn card<'a>(
    source: &'a str,
    (path, occurrence, cfgs, relationships): &'a (&'a str, usize, &'a [String], Vec<usize>),
) -> kaalang_svg::TypeCard<'a> {
    kaalang_svg::TypeCard {
        source,
        path,
        occurrence: *occurrence,
        header: path,
        relationships,
        cfgs,
    }
}

/// The declaration's own `cfg` attributes, as written.
fn cfgs(item: &syn::Item) -> Vec<String> {
    let attributes = match item {
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        _ => return Vec::new(),
    };
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .filter_map(|attribute| syn::spanned::Spanned::span(&attribute.meta).source_text())
        .collect()
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
