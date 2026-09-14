use std::{fs, path::Path, process::Command};

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

/// The break sits between repeating branches, so the expanded cycle has no
/// conforming arrangement.
const IMPOSSIBLE: &str = r#"
#[kaalang]
fn invalid(mode: u8) -> u8 {
    #[cycle("Advance until the mode can leave.")]
    let result = |mut mode| {
        #[choice("Exit or advance?")]
        #[case("Advance from zero.")]
        #[case("Leave the cycle.")]
        #[case("Advance from another mode.")]
        let (first, leave, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Set the mode to one.")]
        |first, &mut mode| *mode = 1;

        |leave, mode| break mode;

        #[action("Set the mode to one.")]
        |last, &mut mode| *mode = 1;
    };

    |result| return result;
}
"#;

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
        .args(["--flow", "invalid", "-o"])
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
            kaalang_svg::render_source(IMPOSSIBLE, "invalid"),
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
fn writes_collapsed_default_and_honors_an_explicit_output() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-collapsed");
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("flow.rs");
    fs::write(&source, CYCLE_SOURCE).unwrap();

    let default = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(&directory)
        .args(["diagram", "flow.rs", "--flow", "route", "--collapse-loops"])
        .output()
        .unwrap();
    assert!(default.status.success(), "{:?}", default.stderr);
    let default_path = directory.join("route_collapsed.svg");
    let default_svg = fs::read_to_string(default_path).unwrap();
    assert!(default_svg.contains(r#"class="node loop""#));
    assert!(!directory.join("route.svg").exists());

    let output = directory.join("chosen.svg");
    let explicit = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .arg("diagram")
        .arg(&source)
        .args(["--flow", "route", "--collapse-loops", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(explicit.status.success(), "{:?}", explicit.stderr);
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        kaalang_svg::render_source_with_options(
            CYCLE_SOURCE,
            "route",
            kaalang_svg::RenderOptions {
                collapse_loops: true,
            },
        )
        .unwrap()
    );

    let invalid_source = directory.join("invalid.rs");
    let preserved_output = directory.join("invalid_collapsed.svg");
    fs::write(&invalid_source, "#[kaalang] fn invalid(input: u8) -> u8 {}").unwrap();
    fs::write(&preserved_output, "keep this diagram").unwrap();
    let invalid = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .current_dir(&directory)
        .args([
            "diagram",
            "invalid.rs",
            "--flow",
            "invalid",
            "--collapse-loops",
        ])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert_eq!(
        fs::read_to_string(preserved_output).unwrap(),
        "keep this diagram"
    );
}
