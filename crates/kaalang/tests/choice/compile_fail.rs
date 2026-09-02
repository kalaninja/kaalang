#[test]
fn invalid_choice_blocks_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/choice/compile_fail/*.rs");
}
