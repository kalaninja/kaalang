//! Every `compile_fail` fixture documents one rejected program and its
//! diagnostic, checked against the `.stderr` beside it.

#[test]
fn rejected_programs_fail_to_compile_with_their_documented_diagnostics() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/*/compile_fail/*.rs");
}
