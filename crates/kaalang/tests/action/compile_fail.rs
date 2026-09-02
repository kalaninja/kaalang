#[test]
fn invalid_action_blocks_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/action/compile_fail/*.rs");
}
