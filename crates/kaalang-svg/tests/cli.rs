use std::{fs, path::Path, process::Command};

const SOURCE: &str = include_str!("fixtures/all_blocks.rs");

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
