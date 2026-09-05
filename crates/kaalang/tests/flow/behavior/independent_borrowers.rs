use kaalang::kaalang;

#[kaalang]
fn independent_borrowers(input: u32) -> (u32, u32) {
    #[action("Produce the first result.")]
    |&input| -> first { *input };

    #[action("Produce the second result.")]
    |&input| -> second { *input + 1 };

    #[end]
    |first, second| {};
}

#[test]
fn two_borrowers_of_one_wire_may_be_ready_together() {
    assert_eq!(independent_borrowers(1), (1, 2));
}
