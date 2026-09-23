use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SOURCE: &str = r"
#[kaalang]
fn route(request: u8) -> u8 {
    |request| return request;
}
";

const CYCLE_SOURCE: &str = r#"
#[kaalang]
fn route(request: u8) -> u8 {
    #[cycle("Use the request.")]
    let response = |request| {
        #[action("Copy the request.")]
        let ready = |request| request;

        |ready| break ready;
    };

    |response| return response;
}
"#;

/// A fresh directory named for one test, holding `source` as `flow.rs`. Cargo
/// reserves this target directory for integration tests.
fn directory(name: &str, source: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("the test directory is writable");
    fs::write(directory.join("flow.rs"), source).expect("the test directory is writable");
    directory
}

/// Runs the binary from `directory` with a command line of plain words.
fn run(directory: &Path, line: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(directory)
        .args(line.split_whitespace())
        .output()
        .expect("the binary runs")
}

#[test]
fn default_output_is_the_flow_name_in_the_current_directory() {
    let directory = directory("cli-default", SOURCE);
    let rendered = run(&directory, "diagram flow.rs --flow route");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert!(directory.join("route.svg").is_file());
}

#[test]
fn explicit_output_replaces_an_existing_file() {
    let directory = directory("cli-explicit", SOURCE);
    fs::write(directory.join("chosen.svg"), "stale diagram").unwrap();
    let rendered = run(&directory, "diagram flow.rs --flow route -o chosen.svg");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert_eq!(
        fs::read_to_string(directory.join("chosen.svg")).unwrap(),
        kaalang_svg::render_source(SOURCE, "route").unwrap()
    );
}

#[test]
fn failed_render_keeps_the_existing_output() {
    let directory = directory("cli-failed", SOURCE);
    fs::write(directory.join("chosen.svg"), "keep this diagram").unwrap();
    let failed = run(&directory, "diagram flow.rs --flow missing -o chosen.svg");
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
    let directory = directory("cli-collapsed", CYCLE_SOURCE);
    let rendered = run(&directory, "diagram flow.rs --flow route --collapse-loops");
    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    let svg = fs::read_to_string(directory.join("route_collapsed.svg")).unwrap();
    assert!(svg.contains(r#"class="node loop""#));
    assert!(!directory.join("route.svg").exists());
}
