#[test]
fn invalid_graph_syntax_fails_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
