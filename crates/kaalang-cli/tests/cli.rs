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
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&directory);
    let members = "[workspace]\nmembers = [\"first\", \"second\"]\nresolver = \"3\"\n";
    write(&directory, "Cargo.toml", members);
    for member in ["first", "second"] {
        write(
            &directory,
            &format!("{member}/Cargo.toml"),
            &manifest(member),
        );
        write(&directory, &format!("{member}/src/lib.rs"), SOURCE);
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
    Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(directory)
        .env("NO_COLOR", "1")
        .args(line.split_whitespace())
        .output()
        .expect("the binary runs")
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
fn modules_off_the_path_are_not_read() {
    let directory = package("cli-off-path", &format!("mod broken;\n{SOURCE}"));
    write(&directory, "src/broken.rs", "fn (");
    let rendered = run(&directory, "diagram --flow route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let stderr = String::from_utf8_lossy(&rendered.stderr);
    assert!(!stderr.contains("broken.rs"), "{stderr}");
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
    let failed = run(&directory, "diagram -p missing --flow route");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(
        stderr.contains("package `missing` is not a member"),
        "{stderr}"
    );
}
