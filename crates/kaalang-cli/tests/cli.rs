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

    // A failed flow never touches its output, so a stale diagram survives. A
    // topology with no conforming diagram is an authored-flow error, and reaches
    // the command line the way every other one does: in the model's own wording
    // rather than as a rendering failure.
    for (file, contents, flow, reported) in [
        ("flow.rs", SOURCE, "missing", "`missing` was not found"),
        (
            "invalid.rs",
            "#[kaalang] fn invalid(input: u8) -> u8 {}",
            "invalid",
            "invalid kaalang flow `invalid`",
        ),
        (
            "impossible.rs",
            IMPOSSIBLE,
            "invalid",
            "could not construct a diagram under RFC 0002",
        ),
        (
            "invalid-label.rs",
            "#[kaalang]\nfn invalid(input: /*\0*/ u8) -> u8 {\n    |input| return input;\n}\n",
            "invalid",
            "contains XML-incompatible character U+0000",
        ),
    ] {
        fs::write(directory.join(file), contents).unwrap();
        let preserved = directory.join(format!("{file}.svg"));
        fs::write(&preserved, "keep this diagram").unwrap();
        let failed = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
            .current_dir(&directory)
            .args(["diagram", file, "--flow", flow, "-o"])
            .arg(&preserved)
            .output()
            .unwrap();
        assert!(!failed.status.success(), "{file}");
        let stderr = String::from_utf8_lossy(&failed.stderr);
        assert!(stderr.contains(reported), "{file}: {stderr}");
        assert_eq!(
            fs::read_to_string(&preserved).unwrap(),
            "keep this diagram",
            "{file}"
        );
    }
    assert!(
        matches!(
            kaalang_svg::render_source(IMPOSSIBLE, "invalid"),
            Err(kaalang_svg::RenderError::InvalidFlow { .. })
        ),
        "the library should reject it as an invalid flow, not an unroutable one"
    );
}

#[test]
fn renders_source_with_parser_prefixes() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-prefixes");
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("flow.rs");
    let output = directory.join("flow.svg");
    fs::write(&source, format!("\u{feff}#!/usr/bin/env rustx\r\n{SOURCE}")).unwrap();

    let rendered = Command::new(env!("CARGO_BIN_EXE_cargo-kaalang"))
        .args(["diagram"])
        .arg(&source)
        .args(["--flow", "route", "-o"])
        .arg(&output)
        .output()
        .unwrap();

    assert!(rendered.status.success(), "{:?}", rendered.stderr);
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        kaalang_svg::render_source(SOURCE, "route").unwrap()
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
