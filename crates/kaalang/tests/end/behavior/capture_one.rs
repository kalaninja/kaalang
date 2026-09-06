use kaalang::kaalang;

#[kaalang]
fn capture_one(input: u32) -> u32 {
    #[action("Increment the input.")]
    |input| -> result { input + 1 };
}

#[test]
fn end_returns_one_complete_wire_value() {
    assert_eq!(capture_one(1), 2);
}
