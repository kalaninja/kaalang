#[test]
fn invalid_question_blocks_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/question/compile_fail/*.rs");
}
