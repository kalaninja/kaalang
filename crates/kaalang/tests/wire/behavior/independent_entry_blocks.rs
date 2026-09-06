use kaalang::kaalang;

#[kaalang]
fn independent_entry_blocks(condition: bool) -> (u32, u32) {
    #[question("Choose the pair.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes pair.")]
    |yes| -> (left, right) { (1, 2) };

    #[action("Build the no pair.")]
    |no| -> (left, right) { (3, 4) };

    #[action("Use the left value.")]
    |left| -> first { left * 10 };

    #[action("Use the right value.")]
    |right| -> second { right * 10 };

    #[action("Pair the entry results.")]
    |first, second| -> result { (first, second) };
}

#[test]
fn one_group_reaches_two_independent_entry_blocks() {
    assert_eq!(independent_entry_blocks(true), (10, 20));
    assert_eq!(independent_entry_blocks(false), (30, 40));
}
