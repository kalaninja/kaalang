use kaalang::kaalang;

fn __kaalang_same_type(value: u8) -> u8 {
    value + 1
}

#[kaalang]
fn type_gate_helper_hygiene(condition: bool) -> u8 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| condition;

    #[action("Call the authored helper for yes.")]
    let (end, _marker) = |yes| (__kaalang_same_type(1), 3u8);

    #[action("Call the authored helper for no.")]
    let (end, _marker) = |no| (__kaalang_same_type(2), 4);
}

#[test]
fn a_generated_type_gate_does_not_shadow_authored_helpers() {
    assert_eq!(type_gate_helper_hygiene(true), 2);
    assert_eq!(type_gate_helper_hygiene(false), 3);
}
