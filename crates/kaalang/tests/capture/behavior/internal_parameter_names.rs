use kaalang::kaalang;

#[kaalang]
fn internal_parameter_names(
    first: u32,
    __kaalang_wire_0: u32,
    r#__kaalang_wire_0_: u32,
) -> (u32, u32, u32) {
    #[action("Keep the inputs.")]
    let result = |first, __kaalang_wire_0, r#__kaalang_wire_0_| {
        (first, __kaalang_wire_0, r#__kaalang_wire_0_)
    };

    |result| return result;
}

#[test]
fn parameter_spellings_cannot_shadow_another_input_during_lowering() {
    assert_eq!(internal_parameter_names(1, 2, 3), (1, 2, 3));
}
