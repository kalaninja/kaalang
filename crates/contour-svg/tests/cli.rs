use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const SOURCE: &str = include_str!("fixtures/all_blocks.rs");

#[test]
fn writes_default_and_explicit_outputs_only_after_success() {
    let directory = TempDirectory::new();
    let source = directory.path.join("flow.rs");
    fs::write(&source, SOURCE).unwrap();

    let default = Command::new(env!("CARGO_BIN_EXE_cargo-contour"))
        .current_dir(&directory.path)
        .args(["diagram", "flow.rs", "--flow", "route"])
        .output()
        .unwrap();
    assert!(default.status.success(), "{:?}", default.stderr);
    assert!(directory.path.join("route.svg").is_file());

    let explicit_path = directory.path.join("nested.svg");
    fs::write(&explicit_path, "stale diagram").unwrap();
    let explicit = Command::new(env!("CARGO_BIN_EXE_cargo-contour"))
        .current_dir(&directory.path)
        .arg("diagram")
        .arg(&source)
        .args(["--flow", "route", "-o"])
        .arg(&explicit_path)
        .output()
        .unwrap();
    assert!(explicit.status.success(), "{:?}", explicit.stderr);
    assert_eq!(fs::read_to_string(explicit_path).unwrap(), render_source());

    let missing_path = directory.path.join("missing.svg");
    let missing = Command::new(env!("CARGO_BIN_EXE_cargo-contour"))
        .current_dir(&directory.path)
        .args(["diagram", "flow.rs", "--flow", "missing"])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(!missing_path.exists());

    let invalid_source = directory.path.join("invalid.rs");
    let invalid_output = directory.path.join("invalid.svg");
    fs::write(&invalid_source, "#[contour] fn invalid(input: u8) -> u8 {}").unwrap();
    let invalid = Command::new(env!("CARGO_BIN_EXE_cargo-contour"))
        .current_dir(&directory.path)
        .arg("diagram")
        .arg(&invalid_source)
        .args(["--flow", "invalid", "-o"])
        .arg(&invalid_output)
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(!invalid_output.exists());

    let preserved_output = directory.path.join("preserved.svg");
    fs::write(&preserved_output, "keep this diagram").unwrap();
    let preserved = Command::new(env!("CARGO_BIN_EXE_cargo-contour"))
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

fn render_source() -> String {
    contour_svg::render_source(SOURCE, "route").expect("the fixture flow renders")
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the system clock is after the Unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("contour-svg-{}-{unique}", std::process::id()));
        fs::create_dir(&path).expect("the temporary directory name is unique");
        Self { path }
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
