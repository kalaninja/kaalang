#[test]
fn invalid_empty_flows_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/empty_flow/compile_fail/*.rs");
}
