use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SOURCE: &str = r"
#[kaa]
fn route(request: u8) -> u8 {
    |request| return request;
}
";

const CYCLE_SOURCE: &str = r#"
#[kaa]
fn route(request: u8) -> u8 {
    #[kaa::cycle("Use the request.")]
    let response = |request| loop {
        #[kaa::action("Copy the request.")]
        let response = |request| request;
    };

    |response| return response;
}
"#;

/// A fresh package named for one test, with `source` as its library root. Its
/// own `[workspace]` keeps it out of the kaalang workspace around the target
/// directory Cargo reserves for integration tests.
fn package(name: &str, source: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&directory);
    write(
        &directory,
        "Cargo.toml",
        &format!("{}\n[workspace]\n", manifest(name)),
    );
    write(&directory, "src/lib.rs", source);
    directory
}

/// A fresh workspace named for one test, whose members `first` and `second`
/// both declare the flow `route`.
fn two_packages(name: &str) -> PathBuf {
    workspace(name, &[("first", SOURCE), ("second", SOURCE)])
}

/// A fresh workspace named for one test, with a member for each package name
/// and its library root.
fn workspace(name: &str, members: &[(&str, &str)]) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&directory);
    let listed: Vec<String> = members
        .iter()
        .map(|(member, _)| format!("\"{member}\""))
        .collect();
    let root = format!(
        "[workspace]\nmembers = [{}]\nresolver = \"3\"\n",
        listed.join(", ")
    );
    write(&directory, "Cargo.toml", &root);
    for (member, source) in members {
        write(
            &directory,
            &format!("{member}/Cargo.toml"),
            &manifest(member),
        );
        write(&directory, &format!("{member}/src/lib.rs"), source);
    }
    directory
}

fn manifest(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n")
}

/// Writes `contents` at `path` inside `directory`, creating its parents.
fn write(directory: &Path, path: &str, contents: &str) {
    let path = directory.join(path);
    fs::create_dir_all(path.parent().expect("the path is inside the directory"))
        .expect("the test directory is writable");
    fs::write(path, contents).expect("the test directory is writable");
}

/// Runs the binary from `directory` with a command line of plain words, with
/// color off so assertions see plain text.
fn run(directory: &Path, line: &str) -> Output {
    run_with(directory, &line.split_whitespace().collect::<Vec<_>>())
}

/// Runs the binary from `directory` with these arguments, with color off.
fn run_with(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(directory)
        .env("NO_COLOR", "1")
        .args(arguments)
        .output()
        .expect("the binary runs")
}

/// A method flow that names `step` in its one action, which reads `receiver`.
fn method(name: &str, receiver: &str, step: &str) -> String {
    format!(
        "#[kaa]\nfn {name}({receiver}) -> u8 {{\n    #[kaa::action(\"{step}\")]\n    let response = |{receiver}| 1;\n\n    |response| return response;\n}}\n"
    )
}

#[test]
fn help_prints_usage_and_succeeds() {
    let directory = package("cli-help", SOURCE);
    for line in ["--help", "-h", "kaalang --help"] {
        let printed = run(&directory, line);
        assert!(printed.status.success(), "{:?}", printed.stderr);
        assert!(String::from_utf8_lossy(&printed.stdout).contains("Usage: cargo kaalang"));
    }
}

#[test]
fn version_prints_the_crate_version() {
    let directory = package("cli-version", SOURCE);
    for line in ["--version", "-V", "kaalang --version"] {
        let printed = run(&directory, line);
        assert!(printed.status.success(), "{:?}", printed.stderr);
        assert_eq!(
            String::from_utf8_lossy(&printed.stdout).trim_end(),
            concat!("cargo-kaalang ", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn default_output_is_the_flow_name_in_the_current_directory() {
    let directory = package("cli-default", SOURCE);
    let rendered = run(&directory, "diagram --flow route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn explicit_output_replaces_an_existing_file() {
    let directory = package("cli-explicit", SOURCE);
    fs::write(directory.join("chosen.svg"), "stale diagram").unwrap();
    let rendered = run(&directory, "diagram --flow route -o chosen.svg");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert_eq!(
        fs::read_to_string(directory.join("chosen.svg")).unwrap(),
        kaalang_svg::render_source(SOURCE, "route").unwrap()
    );
}

#[test]
fn output_cannot_replace_the_source() {
    let directory = package("cli-source-output", SOURCE);
    let failed = run(&directory, "diagram --flow route -o ./src/lib.rs");
    assert!(!failed.status.success());
    assert_eq!(
        fs::read_to_string(directory.join("src/lib.rs")).unwrap(),
        SOURCE
    );
}

#[cfg(unix)]
#[test]
fn output_cannot_replace_the_source_through_a_symlink() {
    let directory = package("cli-source-symlink", SOURCE);
    std::os::unix::fs::symlink("src/lib.rs", directory.join("alias.svg")).unwrap();
    let failed = run(&directory, "diagram --flow route -o alias.svg");
    assert!(!failed.status.success());
    assert_eq!(
        fs::read_to_string(directory.join("src/lib.rs")).unwrap(),
        SOURCE
    );
}

#[test]
fn output_cannot_replace_the_source_through_a_hard_link() {
    let directory = package("cli-source-hard-link", SOURCE);
    fs::hard_link(directory.join("src/lib.rs"), directory.join("alias.svg")).unwrap();
    let failed = run(&directory, "diagram --flow route -o alias.svg");
    assert!(!failed.status.success());
    assert_eq!(
        fs::read_to_string(directory.join("src/lib.rs")).unwrap(),
        SOURCE
    );
}

#[test]
fn failed_render_keeps_the_existing_output() {
    let directory = package("cli-failed", SOURCE);
    fs::write(directory.join("chosen.svg"), "keep this diagram").unwrap();
    let failed = run(&directory, "diagram --flow missing -o chosen.svg");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("`missing` was not found"), "{stderr}");
    assert_eq!(
        fs::read_to_string(directory.join("chosen.svg")).unwrap(),
        "keep this diagram"
    );
}

#[test]
fn collapsed_default_output_has_a_collapsed_suffix() {
    let directory = package("cli-collapsed", CYCLE_SOURCE);
    let rendered = run(&directory, "diagram --flow route --collapse-cycles");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("route_collapsed.svg")).unwrap();
    assert!(svg.contains(r#"class="node cycle""#));
    assert!(!directory.join("route.svg").exists());
}

#[test]
fn flows_in_file_modules_are_found_by_their_path() {
    let directory = package("cli-file-modules", "mod review;\n");
    write(&directory, "src/review/mod.rs", "mod inner;\n");
    write(&directory, "src/review/inner.rs", "mod deeper;\n");
    write(&directory, "src/review/inner/deeper.rs", SOURCE);
    let rendered = run(&directory, "diagram --flow review::inner::deeper::route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn methods_in_inline_modules_are_found_by_their_path() {
    let source = SOURCE.replace("fn route", "pub(crate) fn approve");
    let directory = package(
        "cli-inline-method",
        &format!("mod review {{ pub(crate) struct Report; impl Report {{ {source} }} }}\n"),
    );
    let rendered = run(&directory, "diagram --flow review::Report::approve");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("approve.svg").is_file());
}

#[test]
fn a_path_attribute_names_the_module_file() {
    let directory = package(
        "cli-path-attribute",
        "#[path = \"elsewhere/flows.rs\"]\nmod review;\n",
    );
    write(&directory, "src/elsewhere/flows.rs", SOURCE);
    let rendered = run(&directory, "diagram --flow review::route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn an_inline_module_path_in_a_module_file_is_relative_to_its_directory() {
    let directory = package("cli-inline-path", "mod review;\n");
    write(
        &directory,
        "src/review.rs",
        "#[path = \"elsewhere\"]\nmod inner {\n    mod deeper;\n}\n",
    );
    write(&directory, "src/elsewhere/deeper.rs", SOURCE);
    let rendered = run(&directory, "diagram --flow review::inner::deeper::route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn a_re_exported_item_is_selected_through_its_re_export() {
    let directory = package(
        "cli-re-export",
        &format!(
            "mod review {{\n{}}}\npub use review::route;\nmod model {{ #[kaa] pub struct Member; }}\npub use model::Member;\n",
            SOURCE.replace("fn route", "pub fn route")
        ),
    );
    for (line, file) in [
        ("diagram --flow route", "route.svg"),
        ("diagram --type Member", "Member.svg"),
    ] {
        let rendered = run(&directory, line);
        assert!(rendered.status.success(), "{line}: {:?}", rendered.stderr);
        assert!(directory.join(file).is_file(), "{line}");
    }
}

#[test]
fn a_method_is_selected_through_its_types_path_wherever_its_impl_is() {
    let directory = package(
        "cli-impl-elsewhere",
        &format!(
            "mod model {{ pub struct Report; }}\nmod review {{ impl crate::model::Report {{ {} }} }}\n",
            method("approve", "&self", "Approve.")
        ),
    );
    let rendered = run(&directory, "diagram --flow model::Report::approve");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    // rustc would find no `Report` on these paths.
    for wrong in [
        "review::Report::approve",
        "Report::approve",
        "serde::Report::approve",
    ] {
        let failed = run(&directory, &format!("diagram --flow {wrong}"));
        assert!(!failed.status.success(), "{wrong}");
    }
}

#[test]
fn an_inherent_method_is_selected_over_a_trait_method() {
    let directory = package(
        "cli-inherent-first",
        &format!(
            "struct Report;\ntrait Approve {{ fn approve(&self) -> u8; }}\nimpl Approve for Report {{ {} }}\nimpl Report {{ {} }}\n",
            method("approve", "&self", "From the trait."),
            method("approve", "&self", "Inherent."),
        ),
    );
    let rendered = run(&directory, "diagram --flow Report::approve");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("approve.svg")).unwrap();
    assert!(svg.contains("Inherent."), "{svg}");
}

#[test]
fn a_method_two_traits_provide_fails_listing_both() {
    let directory = package(
        "cli-two-traits",
        &format!(
            "struct Report;\ntrait Approve {{ fn approve(&self) -> u8; }}\ntrait Review {{ fn approve(&self) -> u8; }}\nimpl Approve for Report {{ {} }}\nimpl Review for Report {{ {} }}\n",
            method("approve", "&self", "Approve."),
            method("approve", "&self", "Review."),
        ),
    );
    let failed = run(&directory, "diagram --flow Report::approve");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    for listed in [
        "<Report as Approve>::approve",
        "<Report as Review>::approve",
    ] {
        assert!(stderr.contains(listed), "{stderr}");
    }
}

#[test]
fn a_qualified_path_selects_a_kept_default_method() {
    let directory = package(
        "cli-kept-default",
        &format!(
            "struct Report;\ntrait Approve {{ {} }}\nimpl Approve for Report {{}}\n",
            method("approve", "&self", "Default.")
        ),
    );
    let rendered = run_with(
        &directory,
        &["diagram", "--flow", "<Report as Approve>::approve"],
    );
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("approve.svg").is_file());
}

#[test]
fn a_kept_default_method_of_a_trait_from_another_crate_is_selected() {
    let directory = workspace(
        "cli-default-elsewhere",
        &[
            (
                "app",
                "pub struct Report;\nimpl model::Approve for Report {}\n",
            ),
            (
                "model",
                &format!(
                    "pub trait Approve {{ {} }}\n",
                    method("approve", "&self", "Default.")
                ),
            ),
        ],
    );
    for path in ["<Report as model::Approve>::approve", "Report::approve"] {
        let rendered = run_with(&directory, &["diagram", "--flow", path]);
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
    }
}

#[test]
fn a_qualified_path_selects_the_method_of_a_reference_type() {
    let directory = package(
        "cli-reference-type",
        &format!(
            "struct Report;\ntrait Approve {{ fn approve(self) -> u8; }}\nimpl Approve for &Report {{ {} }}\n",
            method("approve", "self", "Borrowed.")
        ),
    );
    let rendered = run_with(
        &directory,
        &["diagram", "--flow", "<&Report as Approve>::approve"],
    );
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("approve.svg").is_file());
}

#[test]
fn an_unparsable_crate_does_not_stop_the_search() {
    let directory = package("cli-unparsable", SOURCE);
    write(&directory, "tests/broken.rs", "fn (");
    let rendered = run(&directory, "diagram --flow route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn a_found_flow_warns_about_files_that_could_not_be_searched() {
    let directory = package("cli-unparsable-warning", SOURCE);
    write(&directory, "tests/broken.rs", "fn (");
    let rendered = run(&directory, "diagram --flow route");
    let stderr = String::from_utf8_lossy(&rendered.stderr);
    for listed in ["warning", "could not be searched", "broken.rs"] {
        assert!(stderr.contains(listed), "{stderr}");
    }
}

#[test]
fn a_missing_flow_lists_the_files_that_could_not_be_searched() {
    let directory = package("cli-unparsable-missing", "mod broken;\n");
    write(&directory, "src/broken.rs", "fn (");
    let failed = run(&directory, "diagram --flow broken::route");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    for listed in [
        "`broken::route` was not found",
        "could not be searched",
        "broken.rs",
    ] {
        assert!(stderr.contains(listed), "{stderr}");
    }
}

#[test]
fn a_leading_crate_segment_is_accepted() {
    let directory = package("cli-crate-segment", SOURCE);
    let rendered = run(&directory, "diagram --flow crate::route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn render_errors_name_the_requested_path() {
    let directory = package("cli-render-error", "mod review;\n");
    write(
        &directory,
        "src/review.rs",
        "#[kaa]\nfn route(request: u8) -> u8 {}\n",
    );
    let failed = run(&directory, "diagram --flow review::route");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("`review::route`"), "{stderr}");
}

#[test]
fn a_raw_identifier_flow_is_written_without_its_prefix() {
    let directory = package(
        "cli-raw-identifier",
        &SOURCE.replace("fn route", "fn r#match"),
    );
    let rendered = run(&directory, "diagram --flow r#match");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("match.svg").is_file());
}

#[test]
fn a_path_in_two_packages_fails_listing_both() {
    let directory = two_packages("cli-two-packages");
    let failed = run(&directory, "diagram --flow route");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    for listed in ["first: lib `first`", "second: lib `second`", "-p <PACKAGE>"] {
        assert!(stderr.contains(listed), "{stderr}");
    }
    assert!(!directory.join("route.svg").exists());
}

#[test]
fn the_package_option_selects_one_of_two_packages() {
    let directory = two_packages("cli-package-option");
    let rendered = run(&directory, "diagram -p second --flow route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn a_flow_in_a_file_that_two_crates_include_is_found_once() {
    let directory = package("cli-flow-shared-file", "pub mod shared;\n");
    write(&directory, "src/main.rs", "mod shared;\nfn main() {}\n");
    write(
        &directory,
        "src/shared.rs",
        &SOURCE.replace("fn route", "pub fn route"),
    );
    let rendered = run(&directory, "diagram --flow shared::route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn a_path_in_two_crates_of_one_package_cannot_be_selected() {
    let directory = package("cli-two-crates", SOURCE);
    write(&directory, "tests/route.rs", SOURCE);
    let failed = run(&directory, "diagram -p cli-two-crates --flow route");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    for listed in ["lib `cli_two_crates`", "test `route`"] {
        assert!(stderr.contains(listed), "{stderr}");
    }
    assert!(!stderr.contains("-p <PACKAGE>"), "{stderr}");
}

#[test]
fn the_package_hint_needs_a_package_that_declares_the_path_once() {
    let directory = two_packages("cli-no-package-hint");
    for member in ["first", "second"] {
        write(&directory, &format!("{member}/tests/route.rs"), SOURCE);
    }
    let failed = run(&directory, "diagram --flow route");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("test `route`"), "{stderr}");
    assert!(!stderr.contains("-p <PACKAGE>"), "{stderr}");
}

#[test]
fn an_unknown_package_fails() {
    let directory = package("cli-unknown-package", SOURCE);
    for line in [
        "diagram -p missing --flow route",
        "diagram -p missing --type Member",
    ] {
        let failed = run(&directory, line);
        assert!(!failed.status.success(), "{line}");
        let stderr = String::from_utf8_lossy(&failed.stderr);
        assert!(
            stderr.contains("package `missing` is not a member"),
            "{stderr}"
        );
    }
}

#[test]
fn a_module_that_includes_its_own_file_is_read_once() {
    let directory = package(
        "cli-path-loop",
        "pub struct Member;\n#[path = \"lib.rs\"]\nmod again;\n",
    );
    let rendered = run(&directory, "diagram --type Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("Member.svg").is_file());
}

/// The top of each card in `svg`, in drawing order.
fn tops(svg: &str) -> Vec<&str> {
    svg.match_indices(r#"<rect class="card-shape" x=""#)
        .map(|(at, _)| {
            let rest = &svg[at..];
            let y = &rest[rest.find(" y=\"").expect("a card has a top") + 4..];
            &y[..y.find('"').expect("the top is quoted")]
        })
        .collect()
}

/// The positions in `svg` of each of `names`, which must all occur.
fn positions(svg: &str, names: &[&str]) -> Vec<usize> {
    names
        .iter()
        .map(|name| {
            svg.find(name)
                .unwrap_or_else(|| panic!("{name} is not drawn"))
        })
        .collect()
}

#[test]
fn a_type_is_drawn_with_the_types_that_refer_to_it() {
    let directory = package("cli-type", "mod model;\nmod review;\n");
    write(
        &directory,
        "src/model.rs",
        "#[kaa(\"A member.\")]\npub struct Member {\n    name: String,\n}\n",
    );
    write(
        &directory,
        "src/review.rs",
        "use crate::model::Member as Person;\n\n#[kaa(\"A report.\")]\nstruct Report {\n    #[kaa::refer(Person, \"The author.\")]\n    author: u32,\n}\n",
    );
    let rendered = run(&directory, "diagram --type model::Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    positions(&svg, &["review::Report", "model::Member"]);
    assert!(svg.contains("relationship-route"));
}

#[test]
fn a_type_in_an_inline_module_of_a_file_module_is_drawn_by_its_path() {
    let directory = package(
        "cli-type-inline-module",
        "mod model;\n\n#[kaa]\nstruct Report {\n    #[kaa::refer(model::inner::Member, \"m\")]\n    m: u8,\n}\n",
    );
    write(
        &directory,
        "src/model.rs",
        "pub mod inner {\n    pub struct Member;\n}\n",
    );
    let rendered = run(&directory, "diagram --type model::inner::Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    positions(&svg, &["Report", "model::inner::Member"]);
}

#[test]
fn every_declaration_of_a_type_under_exclusive_cfgs_is_drawn() {
    let handle = |cfg: &str, ty: &str| {
        format!(
            "#[cfg({cfg})]\n#[kaa]\nstruct Handle {{\n    #[kaa::refer(Member, \"m\")]\n    m: {ty},\n}}\n"
        )
    };
    let directory = package(
        "cli-type-cfg-twin",
        &format!(
            "pub struct Member;\n{}{}#[kaa]\nstruct Session {{\n    #[kaa::refer(Handle, \"h\")]\n    h: u8,\n}}\n",
            handle("unix", "u8"),
            handle("windows", "u16"),
        ),
    );

    let rendered = run(&directory, "diagram --type Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(rendered.stderr.is_empty(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    positions(
        &svg,
        &[
            "struct · repr(Rust) · cfg(unix)",
            "struct · repr(Rust) · cfg(windows)",
        ],
    );
    assert_eq!(svg.matches("class=\"relationship\"").count(), 2, "{svg}");

    // As the focus, the declarations stand one under another, and the lines
    // go into the first.
    let rendered = run(&directory, "diagram --type Handle");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Handle.svg")).unwrap();
    assert_eq!(svg.matches("class=\"card focus\"").count(), 2, "{svg}");
    assert_eq!(svg.matches("class=\"relationship\"").count(), 1, "{svg}");
}

#[test]
fn declarations_in_modules_under_exclusive_cfgs_show_the_module_cfgs() {
    let handle = |ty: &str| {
        format!(
            "#[kaa]\npub struct Handle {{\n    #[kaa::refer(crate::Member, \"m\")]\n    m: {ty},\n}}\n"
        )
    };
    let directory = package(
        "cli-type-cfg-modules",
        &format!(
            "pub struct Member;\n#[cfg(unix)]\n#[path = \"unix.rs\"]\nmod sys;\n#[cfg(windows)]\n#[path = \"windows.rs\"]\nmod sys;\n#[cfg(unix)]\nmod inline {{\n{}}}\n#[cfg(windows)]\nmod inline {{\n{}}}\n",
            handle("u8"),
            handle("u16"),
        ),
    );
    write(&directory, "src/unix.rs", &handle("u8"));
    write(&directory, "src/windows.rs", &handle("u16"));
    let unix = "struct · repr(Rust) · cfg(unix)";
    let windows = "struct · repr(Rust) · cfg(windows)";

    for (path, file, relationships) in [
        ("Member", "Member.svg", 4),
        ("sys::Handle", "Handle.svg", 0),
        ("inline::Handle", "Handle.svg", 0),
    ] {
        let rendered = run(&directory, &format!("diagram --type {path}"));
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
        let svg = fs::read_to_string(directory.join(file)).unwrap();
        assert_eq!(
            svg.matches(unix).count(),
            svg.matches(windows).count(),
            "{path}: {svg}"
        );
        positions(&svg, &[unix, windows]);
        let drawn = svg.matches("class=\"relationship\"").count();
        assert_eq!(drawn, relationships, "{path}: {svg}");
    }
}

#[test]
fn every_cfg_attr_path_of_a_module_is_read_under_its_cfg() {
    let handle = |ty: &str| {
        format!(
            "#[kaa]\npub struct Handle {{\n    #[kaa::refer(crate::Member, \"m\")]\n    m: {ty},\n}}\n"
        )
    };
    let directory = package(
        "cli-type-cfg-attr-paths",
        "pub struct Member;\n#[cfg_attr(unix, path = \"unix.rs\")]\n#[cfg_attr(windows, path = \"windows.rs\")]\nmod sys;\n",
    );
    write(&directory, "src/unix.rs", &handle("u8"));
    write(&directory, "src/windows.rs", &handle("u16"));
    let unix = "struct · repr(Rust) · cfg(unix)";
    let windows = "struct · repr(Rust) · cfg(windows)";

    for (path, file, relationships) in [
        ("Member", "Member.svg", 2),
        ("sys::Handle", "Handle.svg", 0),
    ] {
        let rendered = run(&directory, &format!("diagram --type {path}"));
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
        let svg = fs::read_to_string(directory.join(file)).unwrap();
        positions(&svg, &[unix, windows]);
        let drawn = svg.matches("class=\"relationship\"").count();
        assert_eq!(drawn, relationships, "{path}: {svg}");
    }
}

#[test]
fn a_file_read_twice_draws_its_type_once() {
    // Both modules read `src/sys/common.rs`, which declares `Report` once.
    let directory = package(
        "cli-type-read-twice",
        "pub struct Member;\n#[cfg(unix)]\nmod sys {\n    pub mod common;\n}\n#[cfg(windows)]\nmod sys {\n    pub mod common;\n}\n",
    );
    write(
        &directory,
        "src/sys/common.rs",
        "#[kaa]\npub struct Report {\n    #[kaa::refer(crate::Member, \"m\")]\n    m: u8,\n}\n",
    );
    for (path, file, class, count) in [
        ("Member", "Member.svg", "class=\"relationship\"", 1),
        (
            "sys::common::Report",
            "Report.svg",
            "class=\"card focus\"",
            1,
        ),
    ] {
        let rendered = run(&directory, &format!("diagram --type {path}"));
        assert!(rendered.status.success(), "{:?}", rendered.stderr);
        assert!(rendered.stderr.is_empty(), "{:?}", rendered.stderr);
        let svg = fs::read_to_string(directory.join(file)).unwrap();
        assert_eq!(svg.matches(class).count(), count, "{path}: {svg}");
    }
}

#[test]
fn a_type_refers_to_itself_as_self() {
    let directory = package(
        "cli-type-self",
        "#[kaa]\nstruct Section {\n    #[kaa::refer(Self, \"s\")]\n    children: Vec<Self>,\n}\n",
    );
    let rendered = run(&directory, "diagram --type Section");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(rendered.stderr.is_empty(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Section.svg")).unwrap();
    assert_eq!(svg.matches("class=\"relationship\"").count(), 1, "{svg}");
}

#[test]
fn a_renamed_dependency_names_its_package_in_a_target() {
    let directory = workspace(
        "cli-type-renamed-dependency",
        &[
            (
                "app",
                "#[kaa]\nstruct FromApp {\n    #[kaa::refer(people::Member, \"m\")]\n    m: u8,\n}\n",
            ),
            ("model-types", "pub struct Member;\n"),
        ],
    );
    write(
        &directory,
        "app/Cargo.toml",
        &format!(
            "{}[dependencies]\npeople = {{ package = \"model-types\", path = \"../model-types\" }}\n",
            manifest("app")
        ),
    );
    let rendered = run(&directory, "diagram --type Member -p model-types");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    positions(&svg, &["app::FromApp", "Member"]);
}

#[test]
fn a_plain_type_may_be_the_focus() {
    let directory = package(
        "cli-plain-type",
        "pub struct Timestamp {\n    seconds: u64,\n}\n\n#[kaa]\nstruct Change {\n    #[kaa::refer(Timestamp, \"When it was saved.\")]\n    saved: Timestamp,\n}\n",
    );
    let rendered = run(&directory, "diagram --type Timestamp");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Timestamp.svg")).unwrap();
    positions(&svg, &["Change", "Timestamp"]);
}

#[test]
fn incoming_types_follow_crate_order() {
    let directory = workspace(
        "cli-type-order",
        &[
            (
                "app",
                "pub struct Member;\n#[kaa]\nstruct FromLib {\n    #[kaa::refer(Member, \"m\")]\n    m: u8,\n}\n",
            ),
            (
                "alpha",
                "#[kaa]\nstruct FromAlpha {\n    #[kaa::refer(app::Member, \"m\")]\n    m: u8,\n}\n",
            ),
        ],
    );
    let refer = |name: &str| {
        format!(
            "#[kaa]\nstruct {name} {{\n    #[kaa::refer(app::Member, \"m\")]\n    m: u8,\n}}\nfn main() {{}}\n"
        )
    };
    write(&directory, "app/tests/check.rs", &refer("FromTest"));
    write(&directory, "app/src/main.rs", &refer("FromBin"));
    let rendered = run(&directory, "diagram --type Member -p app");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    let found = positions(
        &svg,
        &[
            "FromLib<",
            "app::FromBin<",
            "check::FromTest<",
            "alpha::FromAlpha<",
        ],
    );
    assert!(found.is_sorted(), "{found:?}");
    // The library and binary share a row; the tests and the other package each
    // start one.
    let tops = tops(&svg);
    assert_eq!(tops[0], tops[1], "{tops:?}");
    assert!(tops[1] != tops[2] && tops[2] != tops[3], "{tops:?}");
}

#[test]
fn a_file_that_two_crates_include_gives_one_card() {
    let directory = package("cli-type-shared-file", "pub mod model;\n");
    write(&directory, "src/main.rs", "mod model;\nfn main() {}\n");
    write(
        &directory,
        "src/model.rs",
        "pub struct Member;\n#[kaa]\npub struct Report {\n    #[kaa::refer(Member, \"m\")]\n    m: u8,\n}\n",
    );
    let rendered = run(&directory, "diagram --type model::Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    assert_eq!(svg.matches(r#"class="card""#).count(), 1, "{svg}");
}

#[test]
fn targets_that_are_not_types_of_the_workspace_fail_listing_each() {
    let directory = package(
        "cli-type-targets",
        "pub struct Member;\ntrait Named {}\nmod model {}\n\n#[kaa]\nstruct Report {\n    #[kaa::refer(String, \"a\")]\n    a: String,\n    #[kaa::refer(Named, \"b\")]\n    b: u8,\n    #[kaa::refer(model, \"c\")]\n    c: u8,\n}\n",
    );
    let failed = run(&directory, "diagram --type Member");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    for listed in [
        "src/lib.rs:7:18: `String` is not a type of this workspace",
        "src/lib.rs:9:18: `Named` is a trait, not a type",
        "src/lib.rs:11:18: `model` is a module, not a type",
    ] {
        assert!(stderr.contains(listed), "{stderr}");
    }
    assert!(!directory.join("Member.svg").exists());
}

#[test]
fn an_unresolvable_target_warns_and_draws_no_line() {
    let directory = package(
        "cli-type-unresolved",
        "pub struct Member;\ngenerate!();\n\n#[kaa]\nstruct Report {\n    #[kaa::refer(Generated, \"g\")]\n    g: u8,\n}\n",
    );
    let rendered = run(&directory, "diagram --type Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let stderr = String::from_utf8_lossy(&rendered.stderr);
    for listed in ["warning", "`Generated`", "item macro"] {
        assert!(stderr.contains(listed), "{stderr}");
    }
}

#[test]
fn a_target_in_a_file_that_could_not_be_read_warns_and_draws_no_line() {
    let directory = package(
        "cli-type-skipped-target",
        "mod broken;\npub struct Member;\n\n#[kaa]\nstruct Report {\n    #[kaa::refer(broken::Thing, \"t\")]\n    t: u8,\n}\n",
    );
    write(&directory, "src/broken.rs", "fn (");
    let rendered = run(&directory, "diagram --type Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let stderr = String::from_utf8_lossy(&rendered.stderr);
    for listed in ["warning", "broken.rs", "`broken::Thing`"] {
        assert!(stderr.contains(listed), "{stderr}");
    }
}

#[test]
fn a_type_path_in_two_packages_fails_until_one_is_selected() {
    let directory = workspace(
        "cli-type-two-packages",
        &[
            ("first", "pub struct Member;\n"),
            ("second", "pub struct Member;\n"),
        ],
    );
    let failed = run(&directory, "diagram --type Member");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    for listed in ["first: lib `first`", "second: lib `second`", "-p <PACKAGE>"] {
        assert!(stderr.contains(listed), "{stderr}");
    }
    let rendered = run(&directory, "diagram -p second --type Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
}

#[test]
fn the_type_option_does_not_find_a_flow() {
    let directory = package("cli-type-flow", SOURCE);
    let failed = run(&directory, "diagram --type route");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("type `route` was not found"), "{stderr}");
}

#[test]
fn the_column_option_sets_how_many_incoming_types_a_row_holds() {
    let refer = |name: &str| {
        format!("#[kaa]\nstruct {name} {{\n    #[kaa::refer(Member, \"m\")]\n    m: u8,\n}}\n")
    };
    let source: String = ["First", "Second", "Third"]
        .iter()
        .map(|name| refer(name))
        .collect();
    let directory = package("cli-type-columns", &format!("pub struct Member;\n{source}"));
    let rendered = run(&directory, "diagram --type Member --columns 1");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    // Three incoming cards and the focus, each on its own row.
    let tops: std::collections::BTreeSet<&str> = tops(&svg).into_iter().collect();
    assert_eq!(tops.len(), 4, "{tops:?}");
}
