#[test]
fn invalid_flows_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/flow/compile_fail/*.rs");
}
