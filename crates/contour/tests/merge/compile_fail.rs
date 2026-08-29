#[test]
fn invalid_merge_blocks_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/merge/compile_fail/*.rs");
}
