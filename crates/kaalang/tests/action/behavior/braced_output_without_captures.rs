use kaalang::kaa;

#[kaa]
fn braced_output_without_captures() -> u32 {
    #[kaa::action("Build the value.")]
    let value = { 40 + 2 };

    |value| return value;
}

#[test]
fn a_braced_body_without_captures_binds_its_value() {
    assert_eq!(braced_output_without_captures(), 42);
}
