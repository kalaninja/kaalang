use std::{
    fmt::Write as _,
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
        kaalang_svg::render_flow(SOURCE, 0, kaalang_svg::RenderOptions::default()).unwrap()
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
fn rust_2015_imports_and_absolute_queries_start_at_the_crate_root() {
    let directory = package(
        "cli-rust-2015-imports",
        "pub mod model { pub struct Member; }
         pub mod outer { pub use model::Member; }",
    );
    write(
        &directory,
        "Cargo.toml",
        &format!(
            "{}\n[workspace]\n",
            manifest("cli-rust-2015-imports").replace("2024", "2015")
        ),
    );
    let rendered = run(&directory, "diagram --type ::outer::Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("Member.svg").is_file());
}

#[test]
fn ambiguous_globs_do_not_select_a_type_or_flow() {
    let source = SOURCE.replace("fn route", "pub fn route");
    let directory = package(
        "cli-ambiguous-globs",
        &format!(
            "mod left {{ pub struct Member; {source} }}
                  mod right {{ pub struct Member; {source} }}
                  use left::*;
                  use right::*;"
        ),
    );
    for (path, output) in [
        ("--type Member", "Member.svg"),
        ("--flow route", "route.svg"),
    ] {
        let failed = run(&directory, &format!("diagram {path}"));
        assert!(!failed.status.success(), "{path}");
        let stderr = String::from_utf8_lossy(&failed.stderr);
        assert!(
            stderr.contains("ambiguous through glob imports"),
            "{stderr}"
        );
        assert!(!directory.join(output).exists());
    }
}

#[test]
fn a_flow_re_exported_beside_its_module_of_the_same_name_is_selected() {
    let directory = package(
        "cli-flow-named-like-its-module",
        "mod route;\npub use route::route;\n",
    );
    write(
        &directory,
        "src/route.rs",
        &SOURCE.replace("fn route", "pub fn route"),
    );
    let rendered = run(&directory, "diagram --flow route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
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
fn a_method_of_an_impl_through_a_type_alias_is_selected_through_either_name() {
    let directory = package(
        "cli-impl-through-alias",
        &format!(
            "struct Report;\ntype Alias = Report;\nimpl Alias {{ {} }}\n",
            method("approve", "&self", "Approve.")
        ),
    );
    for path in ["Report::approve", "Alias::approve"] {
        let rendered = run(&directory, &format!("diagram --flow {path}"));
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
    }
}

#[test]
fn aliases_keep_their_arguments_and_scopes_when_selecting_methods() {
    let directory = package(
        "cli-alias-arguments",
        &format!(
            r"
        struct Report<T>(T);
        type ByteReport = Report<u8>;
        type Forward<T> = Report<T>;
        type DefaultReport<T = u8> = Forward<T>;
        mod model {{ pub struct Payload; pub type Local = super::Report<Payload>; }}
        struct Payload;
        impl Report<u8> {{ {} }}
        impl Report<u16> {{ {} }}
        impl Report<model::Payload> {{ {} }}
        impl Report<Payload> {{ {} }}
    ",
            method("approve", "&self", "Byte."),
            method("approve", "&self", "Word."),
            method("approve", "&self", "Model."),
            method("approve", "&self", "Root."),
        ),
    );
    for (path, description) in [
        ("ByteReport::approve", "Byte."),
        ("Forward::<u16>::approve", "Word."),
        ("DefaultReport::approve", "Byte."),
        ("model::Local::approve", "Model."),
        ("Forward::<model::Payload>::approve", "Model."),
        ("Forward::<Payload>::approve", "Root."),
    ] {
        let rendered = run_with(&directory, &["diagram", "--flow", path]);
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
        let svg = fs::read_to_string(directory.join("approve.svg")).unwrap();
        assert!(svg.contains(description), "{path}: {svg}");
    }
}

#[test]
fn aliases_expand_on_both_sides_and_inside_structural_types() {
    let directory = package(
        "cli-alias-structural",
        &format!(
            r"
        struct Report<T>(T);
        type ByteReport = Report<u8>;
        type Slice<const N: usize> = [ByteReport; N];
        type Borrowed<'a> = &'a ByteReport;
        trait Review {{ fn approve(self) -> u8; }}
        impl ByteReport {{ {} }}
        impl Review for Slice<2> {{ {} }}
        impl Review for Slice<3> {{ {} }}
        impl Review for Borrowed<'_> {{ {} }}
    ",
            method("approve", "&self", "Byte."),
            method("approve", "self", "Pair."),
            method("approve", "self", "Triple."),
            method("approve", "self", "Borrowed."),
        ),
    );
    for (path, description) in [
        ("Report::<u8>::approve", "Byte."),
        ("<Slice<2> as Review>::approve", "Pair."),
        ("<[Report<u8>; 3] as Review>::approve", "Triple."),
        ("<&Report<u8> as Review>::approve", "Borrowed."),
    ] {
        let rendered = run_with(&directory, &["diagram", "--flow", path]);
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
        let svg = fs::read_to_string(directory.join("approve.svg")).unwrap();
        assert!(svg.contains(description), "{path}: {svg}");
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
    write(
        &directory,
        "app/Cargo.toml",
        &format!(
            "{}[dependencies]\nmodel = {{ path = \"../model\" }}\n",
            manifest("app")
        ),
    );
    for path in ["<Report as model::Approve>::approve", "Report::approve"] {
        let rendered = run_with(&directory, &["diagram", "--flow", path]);
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
    }
}

#[test]
fn an_ambiguous_flow_is_listed_under_the_package_its_path_is_resolved_from() {
    let directory = workspace(
        "cli-ambiguous-elsewhere",
        &[
            (
                "app",
                &format!(
                    "pub struct Report;\npub trait Review {{ fn approve(&self) -> u8; }}\nimpl model::Approve for Report {{}}\nimpl Review for Report {{ {} }}\n",
                    method("approve", "&self", "Reviewed.")
                ),
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
    write(
        &directory,
        "app/Cargo.toml",
        &format!(
            "{}[dependencies]\nmodel = {{ path = \"../model\" }}\n",
            manifest("app")
        ),
    );
    let failed = run(&directory, "diagram --flow Report::approve");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("app: lib `app`"), "{stderr}");
    assert!(!stderr.contains("model: lib"), "{stderr}");
    assert!(!stderr.contains("-p <PACKAGE>"), "{stderr}");
}

#[test]
fn a_method_of_an_impl_for_any_type_is_selected_through_a_type_only() {
    let directory = package(
        "cli-blanket-impl",
        &format!(
            "pub struct Report;\npub mod nested {{}}\npub trait Approve {{ fn approve(&self) -> u8; }}\npub trait Review {{ fn approve(&self) -> u8; }}\nimpl<T> Review for T {{ {} }}\n",
            method("approve", "&self", "Any.")
        ),
    );
    let rendered = run(&directory, "diagram --flow Report::approve");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    not_found(&directory, "diagram --flow Approve::approve");
    not_found(&directory, "diagram --flow nested::approve");
    not_found(&directory, "diagram --flow Missing::approve");
    for path in [
        "<nested>::approve",
        "<Approve>::approve",
        "<Missing as Review>::approve",
        "<&Missing as Review>::approve",
        "<crate::Missing as Review>::approve",
        "<&crate::Missing as Review>::approve",
        "<&nested as Review>::approve",
    ] {
        let failed = run_with(&directory, &["diagram", "--flow", path]);
        assert!(!failed.status.success(), "{path}");
        let stderr = String::from_utf8_lossy(&failed.stderr);
        assert!(stderr.contains("was not found"), "{path}: {stderr}");
    }
}

#[test]
fn trait_object_arguments_select_the_matching_flow() {
    let variants = [
        ("&dyn Fn() -> u8", "No input."),
        ("&dyn Fn(u8) -> u8", "Byte input."),
        ("&dyn Fn(u16) -> u8", "Word input."),
        ("&dyn Fn(u8) -> u16", "Word output."),
        ("&dyn Iterator<Item = u8>", "Byte item."),
        ("&dyn Iterator<Item = u16>", "Word item."),
    ];
    let mut source = "type Byte = u8;\ntrait Run { fn run(&self) -> u8; }\n".to_owned();
    for (ty, description) in variants {
        writeln!(
            source,
            "impl Run for {ty} {{ {} }}",
            method("run", "&self", description),
        )
        .unwrap();
    }
    let directory = package("cli-trait-object-arguments", &source);
    for (ty, description) in variants.into_iter().chain([
        ("&dyn Fn(Byte) -> u8", "Byte input."),
        ("&dyn Iterator<Item = Byte>", "Byte item."),
    ]) {
        let path = format!("<{ty} as Run>::run");
        let rendered = run_with(&directory, &["diagram", "--flow", &path]);
        assert!(rendered.status.success(), "{path}: {:?}", rendered.stderr);
        let svg = fs::read_to_string(directory.join("run.svg")).unwrap();
        for (_, label) in variants {
            assert_eq!(svg.contains(label), label == description, "{path}: {label}");
        }
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
    positions(&svg, &[">review::Report</title>", ">model::Member</title>"]);
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
    positions(&svg, &[">Report</title>", ">model::inner::Member</title>"]);
}

#[test]
fn type_headers_shorten_each_path_against_the_focus_module() {
    let directory = package(
        "cli-type-short-headers",
        r#"
pub mod shop {
    pub mod orders {
        #[kaa]
        pub struct Order {
            #[kaa::refer(Self, "The previous order.")]
            previous: Option<Box<Self>>,
        }
        #[kaa]
        pub struct Line {
            #[kaa::refer(Order, "The order.")]
            order: u32,
        }
        pub mod archive {
            #[kaa]
            pub struct Entry<T> {
                #[kaa::refer(super::Order, "The archived order.")]
                order: u32,
                payload: T,
            }
        }
    }
    pub mod payments {
        #[kaa]
        pub struct Payment {
            #[kaa::refer(super::orders::Order, "The paid order.")]
            order: u32,
        }
    }
    pub mod ordering {
        #[kaa]
        pub struct Note {
            #[kaa::refer(super::orders::Order, "The order.")]
            order: u32,
        }
    }
    #[kaa]
    pub struct Summary {
        #[kaa::refer(orders::Order, "The order.")]
        order: u32,
    }
}
pub mod shopping {
    #[kaa]
    pub struct Receipt {
        #[kaa::refer(crate::shop::orders::Order, "The order.")]
        order: u32,
    }
}
"#,
    );
    let rendered = run(&directory, "diagram --type shop::orders::Order");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Order.svg")).unwrap();
    positions(
        &svg,
        &[
            ">shop::orders::Order</title>",
            ">Line</title>",
            ">archive::Entry&lt;T&gt;</title>",
            ">payments::Payment</title>",
            ">ordering::Note</title>",
            ">Summary</title>",
            ">shopping::Receipt</title>",
            ">shop::orders::Order → shop::orders::Order</title>",
        ],
    );
    assert!(svg.contains("crate::shop::orders::Order"), "{svg}");
}

#[test]
fn type_headers_restore_module_segments_until_collisions_are_resolved() {
    let directory = workspace(
        "cli-type-header-collisions",
        &[
            (
                "app",
                r#"
pub mod shop {
    pub mod orders {
        pub struct Order;
        #[kaa]
        pub struct Payment {
            #[kaa::refer(Order, "The order.")]
            order: u32,
        }
        pub mod external {
            #[kaa]
            pub struct Payment {
                #[kaa::refer(super::Order, "The order.")]
                order: u32,
            }
        }
        pub mod shop {
            pub mod orders {
                #[kaa]
                pub struct Order {
                    #[kaa::refer(crate::shop::orders::Order, "The original order.")]
                    original: u32,
                }
            }
        }
    }
    #[kaa]
    pub struct Payment {
        #[kaa::refer(orders::Order, "The order.")]
        order: u32,
    }
}
pub mod orders {
    #[kaa]
    pub struct Payment {
        #[kaa::refer(crate::shop::orders::Order, "The order.")]
        order: u32,
    }
}
#[kaa]
pub struct Payment {
    #[kaa::refer(shop::orders::Order, "The order.")]
    order: u32,
}
"#,
            ),
            (
                "external",
                "#[kaa] pub struct Payment { #[kaa::refer(app::shop::orders::Order, \"The order.\")] order: u32 }",
            ),
        ],
    );
    write(
        &directory,
        "external/Cargo.toml",
        &format!(
            "{}[dependencies]\napp = {{ path = \"../app\" }}\n",
            manifest("external")
        ),
    );
    let rendered = run(&directory, "diagram --type shop::orders::Order -p app");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Order.svg")).unwrap();
    for header in [
        "shop::orders::Order",
        "shop::orders::Payment",
        "orders::external::Payment",
        "orders::shop::orders::Order",
        "shop::Payment",
        "orders::Payment",
        "Payment",
        "external::Payment",
    ] {
        let title = format!("<title xml:space=\"preserve\">{header}</title>");
        assert_eq!(svg.matches(&title).count(), 1, "{header}: {svg}");
    }
}

const LINUX: &str = "x86_64-unknown-linux-gnu";
const WINDOWS: &str = "x86_64-pc-windows-msvc";

/// Draws `item` from `directory` for `target` and returns the diagram.
fn drawn(directory: &Path, item: &str, target: &str) -> String {
    let rendered = run(
        directory,
        &format!("diagram {item} --target {target} -o drawn.svg"),
    );
    assert!(
        rendered.status.success(),
        "{item} for {target}: {:?}",
        rendered.stderr
    );
    fs::read_to_string(directory.join("drawn.svg")).expect("the diagram was written")
}

/// Asserts that drawing `item` from `directory` fails, finding no such type
/// or flow.
fn not_found(directory: &Path, line: &str) {
    let failed = run(directory, line);
    assert!(!failed.status.success(), "{line}");
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("was not found"), "{line}: {stderr}");
}

#[test]
fn the_target_draws_the_declaration_it_compiles() {
    let directory = package(
        "cli-target-declaration",
        "pub struct Member;\n#[cfg(windows)]\n#[kaa]\nstruct Handle {\n    #[kaa::refer(Member, \"m\")]\n    m: isize,\n}\n#[cfg(unix)]\n#[kaa]\nstruct Handle {\n    #[kaa::refer(Member, \"m\")]\n    m: i32,\n}\n",
    );
    for (target, kept, gone) in [(LINUX, ">i32<", ">isize<"), (WINDOWS, ">isize<", ">i32<")] {
        let svg = drawn(&directory, "--type Handle", target);
        assert!(svg.contains(kept) && !svg.contains(gone), "{target}: {svg}");
        assert!(!svg.contains("cfg("), "{target}: {svg}");
        let svg = drawn(&directory, "--type Member", target);
        assert_eq!(
            svg.matches("class=\"relationship\"").count(),
            1,
            "{target}: {svg}"
        );
    }
}

#[test]
fn a_field_the_target_does_not_compile_has_no_tile_or_line() {
    let directory = package(
        "cli-target-field",
        "pub struct Member;\n#[kaa]\nstruct Handle {\n    #[cfg(windows)]\n    #[kaa::refer(Member, \"w\")]\n    w: u16,\n    #[cfg(unix)]\n    #[kaa::refer(Member, \"m\")]\n    m: i32,\n}\n",
    );
    for (target, kept, gone) in [(LINUX, ">i32<", ">u16<"), (WINDOWS, ">u16<", ">i32<")] {
        let svg = drawn(&directory, "--type Handle", target);
        assert!(svg.contains(kept) && !svg.contains(gone), "{target}: {svg}");
        // The line leaves the field the target keeps, whichever comes first.
        let svg = drawn(&directory, "--type Member", target);
        assert_eq!(
            svg.matches("class=\"relationship\"").count(),
            1,
            "{target}: {svg}"
        );
    }
}

#[test]
fn the_target_draws_the_flow_it_compiles() {
    let flow = |name: &str, step: &str| {
        format!(
            "#[kaa]\npub fn {name}(request: u8) -> u8 {{\n    #[kaa::action(\"{step}\")]\n    let response = |request| request;\n\n    |response| return response;\n}}\n"
        )
    };
    let directory = package(
        "cli-target-flow",
        &format!(
            "pub struct Report;\n#[cfg(windows)]\n{}#[cfg(unix)]\n{}#[cfg(windows)]\nimpl Report {{\n{}}}\n#[cfg(unix)]\nimpl Report {{\n{}}}\n",
            flow("route", "Route on Windows."),
            flow("route", "Route on Unix."),
            flow("approve", "Approve on Windows."),
            flow("approve", "Approve on Unix."),
        ),
    );
    for (target, kept, gone) in [(LINUX, "Unix.", "Windows."), (WINDOWS, "Windows.", "Unix.")] {
        for item in ["--flow route", "--flow Report::approve"] {
            let svg = drawn(&directory, item, target);
            assert!(
                svg.contains(kept) && !svg.contains(gone),
                "{item} for {target}: {svg}"
            );
        }
    }
}

#[test]
fn the_target_selects_a_cfg_attr_module_file() {
    let flow = |step: &str| {
        format!(
            "#[kaa]\npub fn route(request: u8) -> u8 {{\n    #[kaa::action(\"{step}\")]\n    let response = |request| request;\n\n    |response| return response;\n}}\n"
        )
    };
    let directory = package(
        "cli-target-cfg-attr-path",
        "#[cfg_attr(unix, path = \"unix.rs\")]\n#[cfg_attr(windows, path = \"windows.rs\")]\nmod sys;\n",
    );
    write(&directory, "src/unix.rs", &flow("On Unix."));
    write(&directory, "src/windows.rs", &flow("On Windows."));
    for (target, step) in [(LINUX, "On Unix."), (WINDOWS, "On Windows.")] {
        let svg = drawn(&directory, "--flow sys::route", target);
        assert!(svg.contains(step), "{target}: {svg}");
    }
}

#[test]
fn the_host_is_the_default_target() {
    let os = std::env::consts::OS;
    let directory = package(
        "cli-target-host",
        &format!(
            "#[cfg(target_os = \"{os}\")]\npub struct Here;\n#[cfg(not(target_os = \"{os}\"))]\npub struct Elsewhere;\n"
        ),
    );
    assert!(run(&directory, "diagram --type Here").status.success());
    not_found(&directory, "diagram --type Elsewhere");
}

#[test]
fn rustflags_cfgs_hold_and_the_encoded_ones_come_first() {
    let directory = package(
        "cli-target-rustflags",
        "#[cfg(kaalang_plain)]\npub struct Plain;\n#[cfg(kaalang_encoded)]\npub struct Encoded;\n",
    );
    let run_with_flags = |ty: &str, encoded: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"));
        command
            .current_dir(&directory)
            .env("NO_COLOR", "1")
            .env("RUSTFLAGS", "--cfg kaalang_plain")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .args(["diagram", "--type", ty]);
        if let Some(encoded) = encoded {
            command.env("CARGO_ENCODED_RUSTFLAGS", encoded);
        }
        command.output().expect("the binary runs").status.success()
    };
    assert!(run_with_flags("Plain", None));
    assert!(!run_with_flags("Encoded", None));
    let encoded = Some("--cfg\u{1f}kaalang_encoded");
    assert!(run_with_flags("Encoded", encoded));
    assert!(!run_with_flags("Plain", encoded));
}

#[test]
fn features_follow_the_cargo_feature_options() {
    let directory = package(
        "cli-target-features",
        "#[cfg(feature = \"audit\")]\npub struct Audited;\n#[cfg(feature = \"extra\")]\npub struct Extra;\n",
    );
    write(
        &directory,
        "Cargo.toml",
        &format!(
            "{}\n[features]\ndefault = [\"audit\"]\naudit = []\nextra = []\n\n[workspace]\n",
            manifest("cli-target-features")
        ),
    );
    for (options, audited, extra) in [
        ("", true, false),
        ("--features extra", true, true),
        ("--no-default-features --features extra", false, true),
        ("--all-features", true, true),
    ] {
        for (ty, drawn) in [("Audited", audited), ("Extra", extra)] {
            let line = format!("diagram --type {ty} {options}");
            if drawn {
                let rendered = run(&directory, &line);
                assert!(rendered.status.success(), "{line}: {:?}", rendered.stderr);
            } else {
                not_found(&directory, &line);
            }
        }
    }
}

#[test]
fn features_given_with_a_package_are_that_packages() {
    let directory = workspace(
        "cli-target-package-features",
        &[
            ("first", "pub struct Member;\n"),
            (
                "second",
                "#[cfg(feature = \"audit\")]\n#[kaa]\npub struct Report {\n    #[kaa::refer(first::Member, \"m\")]\n    m: u8,\n}\n",
            ),
        ],
    );
    write(
        &directory,
        "first/Cargo.toml",
        &format!("{}\n[features]\naudit = []\n", manifest("first")),
    );
    write(
        &directory,
        "second/Cargo.toml",
        &format!(
            "{}\n[dependencies]\nfirst = {{ path = \"../first\" }}\n\n[features]\naudit = []\n",
            manifest("second")
        ),
    );
    for (line, relationships) in [
        ("diagram -p first --type Member --features audit", 0),
        ("diagram -p first --type Member --features second/audit", 1),
    ] {
        let rendered = run(&directory, line);
        assert!(rendered.status.success(), "{line}: {:?}", rendered.stderr);
        let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
        let lines = svg.matches("class=\"relationship\"").count();
        assert_eq!(lines, relationships, "{line}: {svg}");
    }
}

#[test]
fn a_target_whose_required_features_are_off_is_not_read() {
    let directory = package("cli-target-required-features", "");
    write(
        &directory,
        "Cargo.toml",
        &format!(
            "{}\n[features]\nextra = []\n\n[[bin]]\nname = \"extra\"\npath = \"src/main.rs\"\nrequired-features = [\"extra\"]\n\n[workspace]\n",
            manifest("cli-target-required-features")
        ),
    );
    write(
        &directory,
        "src/main.rs",
        "pub struct OnlyWithExtra;\nfn main() {}\n",
    );
    not_found(&directory, "diagram --type OnlyWithExtra");
    let rendered = run(&directory, "diagram --type OnlyWithExtra --features extra");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
}

#[test]
fn dependency_features_gate_targets_by_their_manifest_names() {
    for (dependency, library) in [
        ("model-types", "model_types"),
        ("model-types", "model_core"),
        ("renamed-model", "model_core"),
    ] {
        let directory = workspace(
            &format!("cli-target-required-{dependency}-{library}"),
            &[("model-types", ""), ("app", "")],
        );
        write(
            &directory,
            "model-types/Cargo.toml",
            &format!(
                "{}\n[lib]\nname = \"{library}\"\n\n[features]\nextra = []\n",
                manifest("model-types")
            ),
        );
        let dependencies = if dependency == "model-types" {
            "[dependencies]\nmodel-types = { path = \"../model-types\" }\n"
        } else {
            "[dependencies]\nrenamed-model = { package = \"model-types\", path = \"../model-types\" }\n"
        };
        write(
            &directory,
            "app/Cargo.toml",
            &format!(
                "{}\n{dependencies}\n[features]\nenabled = [\"{dependency}/extra\"]\n\n[[bin]]\nname = \"extra\"\npath = \"src/main.rs\"\nrequired-features = [\"{dependency}/extra\"]\n",
                manifest("app")
            ),
        );
        write(
            &directory,
            "app/src/main.rs",
            "pub struct OnlyWithExtra;\nfn main() {}\n",
        );
        not_found(&directory, "diagram -p app --type OnlyWithExtra");
        let rendered = run(
            &directory,
            "diagram -p app --type OnlyWithExtra --features enabled",
        );
        assert!(
            rendered.status.success(),
            "{dependency}: {:?}",
            rendered.stderr
        );
    }
}

#[test]
fn cfg_test_holds_in_test_targets_only() {
    let directory = package("cli-target-test", "#[cfg(test)]\npub struct InLibrary;\n");
    write(
        &directory,
        "tests/check.rs",
        "#[cfg(test)]\npub struct InTests;\n",
    );
    assert!(run(&directory, "diagram --type InTests").status.success());
    not_found(&directory, "diagram --type InLibrary");
}

#[test]
fn a_procedural_macro_is_read_for_the_host() {
    let os = std::env::consts::OS;
    let directory = package(
        "cli-target-proc-macro",
        &format!("#[cfg(all(proc_macro, target_os = \"{os}\"))]\npub struct OnHost;\n"),
    );
    write(
        &directory,
        "Cargo.toml",
        &format!(
            "{}\n[lib]\nproc-macro = true\n\n[workspace]\n",
            manifest("cli-target-proc-macro")
        ),
    );
    let rendered = run(
        &directory,
        "diagram --type OnHost --target wasm32-unknown-unknown",
    );
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
}

#[test]
fn a_procedural_macros_dependencies_are_read_for_the_host() {
    let os = std::env::consts::OS;
    let directory = workspace(
        "cli-host-dependencies",
        &[
            ("macros", "use bridge::Member;\n"),
            ("bridge", "pub use model::Member;\n"),
            (
                "model",
                &format!(
                    "pub struct Member {{\n\
                     #[cfg(target_os = \"{os}\")] pub host_field: u8,\n\
                     #[cfg(not(target_os = \"{os}\"))] pub target_field: u8,\n\
                     }}\nimpl Member {{\n\
                     #[cfg(target_os = \"{os}\")] {}\n\
                     #[cfg(not(target_os = \"{os}\"))] {}\n}}\n",
                    method("route", "self", "Host method."),
                    method("route", "self", "Target method."),
                ),
            ),
        ],
    );
    write(
        &directory,
        "macros/Cargo.toml",
        &format!(
            "{}[lib]\nproc-macro = true\n\n[target.'cfg(target_os = \"{os}\")'.dependencies]\nbridge = {{ path = \"../bridge\" }}\n",
            manifest("macros")
        ),
    );
    write(
        &directory,
        "bridge/Cargo.toml",
        &format!(
            "{}[target.'cfg(target_os = \"{os}\")'.dependencies]\nmodel = {{ path = \"../model\" }}\n",
            manifest("bridge")
        ),
    );
    for (package, present, absent) in [
        ("macros", "host_field", "target_field"),
        ("model", "target_field", "host_field"),
    ] {
        let svg = drawn(
            &directory,
            &format!("--type Member -p {package}"),
            "wasm32-unknown-unknown",
        );
        assert!(svg.contains(present), "{package}: {svg}");
        assert!(!svg.contains(absent), "{package}: {svg}");
    }
    for (package, present, absent) in [
        ("macros", "Host method.", "Target method."),
        ("model", "Target method.", "Host method."),
    ] {
        let svg = drawn(
            &directory,
            &format!("--flow Member::route -p {package}"),
            "wasm32-unknown-unknown",
        );
        assert!(svg.contains(present), "{package}: {svg}");
        assert!(!svg.contains(absent), "{package}: {svg}");
    }
}

#[test]
fn an_unknown_target_fails() {
    let directory = package("cli-target-unknown", "pub struct Member;\n");
    let failed = run(&directory, "diagram --type Member --target no-such-target");
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("no-such-target"), "{stderr}");
    assert!(!directory.join("Member.svg").exists());
}

#[test]
fn a_use_the_target_does_not_compile_names_no_target() {
    let directory = package(
        "cli-target-use",
        "pub mod unix { pub struct Handle; }\npub mod windows { pub struct Handle; }\n#[cfg(windows)]\nuse windows::Handle;\n#[cfg(unix)]\nuse unix::Handle;\n#[kaa]\nstruct Session {\n    #[kaa::refer(Handle, \"h\")]\n    h: u8,\n}\n",
    );
    for (target, unix, windows) in [(LINUX, 1, 0), (WINDOWS, 0, 1)] {
        for (ty, relationships) in [("unix::Handle", unix), ("windows::Handle", windows)] {
            let svg = drawn(&directory, &format!("--type {ty}"), target);
            let lines = svg.matches("class=\"relationship\"").count();
            assert_eq!(lines, relationships, "{ty} for {target}: {svg}");
        }
    }
}

#[test]
fn a_module_the_target_does_not_compile_is_not_read() {
    let directory = package(
        "cli-target-unread-module",
        "pub struct Member;\n#[cfg(windows)]\nmod broken;\n",
    );
    write(&directory, "src/broken.rs", "this is not Rust");
    let rendered = run(
        &directory,
        &format!("diagram --type Member --target {LINUX}"),
    );
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(rendered.stderr.is_empty(), "{:?}", rendered.stderr);
}

#[test]
fn a_file_read_twice_draws_its_type_once() {
    // Both modules read `src/common.rs`, which declares `Report` once.
    let directory = package(
        "cli-type-read-twice",
        "pub struct Member;\n#[path = \"common.rs\"]\nmod first;\n#[path = \"common.rs\"]\nmod second;\n",
    );
    write(
        &directory,
        "src/common.rs",
        "#[kaa]\npub struct Report {\n    #[kaa::refer(crate::Member, \"m\")]\n    m: u8,\n}\n",
    );
    let rendered = run(&directory, "diagram --type Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(rendered.stderr.is_empty(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    assert_eq!(svg.matches("class=\"relationship\"").count(), 1, "{svg}");
}

#[test]
fn a_file_two_crates_read_under_their_own_cfgs_draws_the_focus_lines_of_its_crate() {
    let directory = package("cli-shared-file-type", "pub mod model;\n");
    write(
        &directory,
        "src/model.rs",
        "#[cfg_attr(test, derive(Clone))]\n#[kaa]\npub struct Node {\n    #[cfg(test)]\n    #[kaa::refer(Node, \"p\")]\n    p: u8,\n}\n",
    );
    write(
        &directory,
        "tests/it.rs",
        "#[path = \"../src/model.rs\"]\nmod model;\n",
    );
    let rendered = run(&directory, "diagram --type model::Node");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Node.svg")).unwrap();
    assert_eq!(svg.matches("class=\"relationship\"").count(), 0, "{svg}");
}

#[test]
fn distinct_conditional_types_in_a_shared_file_are_ambiguous() {
    let directory = package("cli-shared-file-conditional-types", "pub mod model;\n");
    write(
        &directory,
        "src/model.rs",
        "#[cfg(not(test))]\npub struct Member { production: u8 }\n#[cfg(test)]\npub struct Member { testing: u8 }\n",
    );
    write(
        &directory,
        "tests/it.rs",
        "#[path = \"../src/model.rs\"]\nmod model;\n",
    );
    let rendered = run(&directory, "diagram --type model::Member");
    assert!(!rendered.status.success());
    let stderr = String::from_utf8_lossy(&rendered.stderr);
    assert!(
        stderr.contains("multiple types have the path `model::Member`"),
        "{stderr}"
    );
    assert!(
        stderr.contains("lib `cli_shared_file_conditional_types`"),
        "{stderr}"
    );
    assert!(stderr.contains("test `it`"), "{stderr}");
    assert!(!directory.join("Member.svg").exists());
}

#[test]
fn a_conditional_type_does_not_take_another_declarations_method() {
    let directory = package("cli-shared-file-conditional-method", "pub mod model;\n");
    write(
        &directory,
        "src/model.rs",
        &format!(
            "#[cfg(not(test))]\npub struct Member;\n#[cfg(test)]\npub struct Member;\n#[cfg(not(test))]\nimpl Member {{\n{}}}\n",
            method("route", "self", "The production member.")
        ),
    );
    write(
        &directory,
        "tests/it.rs",
        "#[path = \"../src/model.rs\"]\nmod model;\nuse model::Member as Testing;\n",
    );
    not_found(&directory, "diagram --flow Testing::route");
}

#[test]
fn a_flow_two_crates_read_under_their_own_cfgs_is_found_once() {
    let directory = package("cli-shared-file-flow", "pub mod review;\n");
    write(
        &directory,
        "src/review.rs",
        &format!(
            "#[cfg(test)]\n{}{}",
            SOURCE.replace("fn route", "pub fn fixture"),
            SOURCE.replace("fn route", "pub fn route")
        ),
    );
    write(
        &directory,
        "tests/it.rs",
        "#[path = \"../src/review.rs\"]\nmod review;\n",
    );
    let rendered = run(&directory, "diagram --flow review::route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
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
                "mod model {\n#[kaa]\nstruct FromApp {\n    #[kaa::refer(people::model::Member, \"m\")]\n    m: u8,\n}\n}\n",
            ),
            ("model-types", "pub mod model { pub struct Member; }\n"),
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
    let rendered = run(&directory, "diagram --type model::Member -p model-types");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    positions(
        &svg,
        &[">app::model::FromApp</title>", ">model::Member</title>"],
    );
}

#[test]
fn libraries_with_the_same_name_resolve_through_the_actual_dependency() {
    let directory = workspace(
        "cli-library-name-collision",
        &[
            ("first", "pub struct Member { pub first_field: u8 }"),
            ("second", "pub struct Member { pub second_field: u8 }"),
            (
                "app",
                "#[kaa] pub struct Incoming { #[kaa::refer(model::Member, \"The member.\")] pub key: u8 }",
            ),
        ],
    );
    for package in ["first", "second"] {
        write(
            &directory,
            &format!("{package}/Cargo.toml"),
            &format!("{}[lib]\nname = \"model\"\n", manifest(package)),
        );
    }
    write(
        &directory,
        "app/Cargo.toml",
        &format!(
            "{}[dependencies]\nsecond = {{ path = \"../second\" }}\n",
            manifest("app")
        ),
    );
    let rendered = run(&directory, "diagram -p app --type model::Member");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("Member.svg")).unwrap();
    assert!(svg.contains("second_field"), "{svg}");
    assert!(!svg.contains("first_field"), "{svg}");
    assert!(svg.contains("app::Incoming"), "{svg}");
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
fn the_default_output_is_named_without_generic_arguments() {
    let directory = package(
        "cli-generic-output",
        "pub mod model { pub struct Attachment<T>(T); }\n",
    );
    let rendered = run(
        &directory,
        "diagram --type model::Attachment<crate::Payload>",
    );
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("Attachment.svg").is_file());
    let svg = fs::read_to_string(directory.join("Attachment.svg")).unwrap();
    assert!(svg.contains(">model::Attachment&lt;T&gt;</title>"), "{svg}");
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
    write(
        &directory,
        "alpha/Cargo.toml",
        &format!(
            "{}[dependencies]\napp = {{ path = \"../app\" }}\n",
            manifest("alpha")
        ),
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
