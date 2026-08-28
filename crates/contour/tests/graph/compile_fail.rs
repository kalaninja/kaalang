#[test]
fn invalid_graphs_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/graph/compile_fail/*.rs");
}
