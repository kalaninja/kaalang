#[test]
fn invalid_syntax_fails_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/syntax/compile_fail/*.rs");
}
