#[test]
fn invalid_wires_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/wire/compile_fail/*.rs");
}
