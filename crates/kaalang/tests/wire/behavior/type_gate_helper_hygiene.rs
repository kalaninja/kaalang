use kaalang::kaalang;

fn __kaalang_same_type(value: u8) -> u8 {
    value + 1
}

#[kaalang]
fn type_gate_helper_hygiene(condition: bool) -> u8 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Call the authored helper for yes.")]
    |yes| -> (result, _marker) { (__kaalang_same_type(1u8), 3u8) };

    #[action("Call the authored helper for no.")]
    |no| -> (result, _marker) { (__kaalang_same_type(2u8), 4u8) };

    #[end]
    |result| {};
}

#[test]
fn a_generated_type_gate_does_not_shadow_authored_helpers() {
    assert_eq!(type_gate_helper_hygiene(true), 2);
    assert_eq!(type_gate_helper_hygiene(false), 3);
}
