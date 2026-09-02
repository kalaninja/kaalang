use kaalang::kaalang;

#[kaalang]
fn capture_several(input: u32) -> (u32, u32, u32) {
    #[action("Build three values.")]
    |input| -> (first, second, third) { (input, input + 1, input + 2) };

    #[end]
    |third, first, second| {};
}

#[test]
fn end_preserves_authored_input_order() {
    assert_eq!(capture_several(1), (3, 1, 2));
}
