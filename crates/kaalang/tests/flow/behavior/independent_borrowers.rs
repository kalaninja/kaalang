use kaalang::kaalang;

#[kaalang]
fn independent_borrowers(input: u32) -> (u32, u32) {
    #[action("Produce the first value.")]
    |&input| -> first { *input };

    #[action("Produce the second value.")]
    |&input| -> second { *input + 1 };

    #[action("Pair the two values.")]
    |first, second| -> result { (first, second) };
}

#[test]
fn two_borrowers_of_one_wire_may_be_ready_together() {
    assert_eq!(independent_borrowers(1), (1, 2));
}
