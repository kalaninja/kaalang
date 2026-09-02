#[test]
fn invalid_end_blocks_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/end/compile_fail/*.rs");
}
