use std::{fs, path::Path, process::Command};

const SOURCE: &str = r#"
#[kaalang]
fn route(request: u8) -> u8 {
    #[action("Use the request.")]
    let end = |&request| { request };
}
"#;

/// Binary question ports fix the order of the paths between these nested
/// returns, so end cannot remain below both of them.
const IMPOSSIBLE: &str =
    include_str!("../../kaalang/tests/loop/compile_fail/end_enclosed_by_nested_returns.rs");

#[test]
fn writes_default_and_explicit_outputs_only_after_success() {
    // Cargo reserves this directory for integration tests; only this test uses it.
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("flow.rs");
    fs::write(&source, SOURCE).unwrap();

    let default = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(&directory)
        .args(["diagram", "flow.rs", "--flow", "route"])
        .output()
        .unwrap();
    assert!(default.status.success(), "{:?}", default.stderr);
    assert!(directory.join("route.svg").is_file());

    let explicit_path = directory.join("nested.svg");
    fs::write(&explicit_path, "stale diagram").unwrap();
    let explicit = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(&directory)
        .arg("diagram")
        .arg(&source)
        .args(["--flow", "route", "-o"])
        .arg(&explicit_path)
        .output()
        .unwrap();
    assert!(explicit.status.success(), "{:?}", explicit.stderr);
    assert_eq!(
        fs::read_to_string(explicit_path).unwrap(),
        kaalang_svg::render_source(SOURCE, "route").unwrap()
    );

    let missing_path = directory.join("missing.svg");
    let missing = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(&directory)
        .args(["diagram", "flow.rs", "--flow", "missing"])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(!missing_path.exists());

    let invalid_source = directory.join("invalid.rs");
    let invalid_output = directory.join("invalid.svg");
    fs::write(&invalid_source, "#[kaalang] fn invalid(input: u8) -> u8 {}").unwrap();
    let invalid = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(&directory)
        .arg("diagram")
        .arg(&invalid_source)
        .args(["--flow", "invalid", "-o"])
        .arg(&invalid_output)
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(!invalid_output.exists());

    // A topology with no conforming diagram is an authored-flow error, so it
    // reaches the command line the way every other one does: no output, and the
    // model's own wording rather than a rendering failure.
    let impossible_source = directory.join("impossible.rs");
    let impossible_output = directory.join("impossible.svg");
    fs::write(&impossible_source, IMPOSSIBLE).unwrap();
    let impossible = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(&directory)
        .arg("diagram")
        .arg(&impossible_source)
        .args(["--flow", "end_enclosed_by_nested_returns", "-o"])
        .arg(&impossible_output)
        .output()
        .unwrap();
    assert!(!impossible.status.success());
    assert!(!impossible_output.exists());
    let reported = String::from_utf8_lossy(&impossible.stderr);
    assert!(
        reported.contains("could not construct a diagram under RFC 0002"),
        "the command line should report the model's decision: {reported}"
    );
    assert!(
        matches!(
            kaalang_svg::render_source(IMPOSSIBLE, "end_enclosed_by_nested_returns"),
            Err(kaalang_svg::RenderError::InvalidFlow { .. })
        ),
        "the library should reject it as an invalid flow, not an unroutable one"
    );

    let preserved_output = directory.join("preserved.svg");
    fs::write(&preserved_output, "keep this diagram").unwrap();
    let preserved = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .arg("diagram")
        .arg(&invalid_source)
        .args(["--flow", "invalid", "-o"])
        .arg(&preserved_output)
        .output()
        .unwrap();
    assert!(!preserved.status.success());
    assert_eq!(
        fs::read_to_string(preserved_output).unwrap(),
        "keep this diagram"
    );
}

#[test]
fn refuses_an_enclosed_break_before_writing_a_diagram() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-enclosed-break");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("flow.rs");
    fs::write(
        &source,
        include_str!("../../kaalang/tests/loop/compile_fail/enclosed_break.rs"),
    )
    .unwrap();
    let output = directory.join("diagram.svg");
    fs::write(&output, "keep this diagram").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .arg("diagram")
        .arg(&source)
        .args(["--flow", "invalid", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("no conforming arrangement"));
    assert_eq!(fs::read_to_string(output).unwrap(), "keep this diagram");
}
