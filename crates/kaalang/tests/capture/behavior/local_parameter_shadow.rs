use kaalang::kaalang;

#[kaalang]
fn local_parameter_shadow(value: u32) -> (u32, u32) {
    #[action("Use an explicit local binding.")]
    let result = || {
        let value = 9;
        value + 1
    };

    |value, result| return (value, result);
}

#[test]
fn a_local_binding_can_shadow_a_parameter_spelling() {
    assert_eq!(local_parameter_shadow(1), (1, 10));
}
