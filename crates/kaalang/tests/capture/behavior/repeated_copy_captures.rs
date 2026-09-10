use kaalang::kaalang;

/// Two blocks capture the same `Copy` wire by value. kaalang orders them by
/// source position and leaves the copy to Rust.
#[kaalang]
fn repeated_copy_captures(input: u32) -> (u32, u32) {
    #[action("Consume the input.")]
    let first = |input| input;

    #[action("Consume the input again.")]
    let second = |input| input + 1;

    #[action("Pair the two values.")]
    let end = |first, second| (first, second);
}

#[test]
fn a_copy_wire_reaches_both_of_its_consumers() {
    assert_eq!(repeated_copy_captures(3), (3, 4));
}
