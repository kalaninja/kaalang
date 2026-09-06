use kaalang::kaalang;

#[kaalang]
fn question_after_one_entry_block(condition: bool) -> u32 {
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

    #[question("Is the first value large?")]
    |first| -> (large, small) { first > 10 };

    #[action("Combine a large first value with the second.")]
    |large, second| -> result { second + 100 };

    #[action("Combine a small first value with the second.")]
    |small, second| -> result { second };

    #[end]
    |result| {};
}

#[test]
fn a_question_after_one_entry_block_selects_consumers_of_the_other() {
    assert_eq!(question_after_one_entry_block(true), 20);
    assert_eq!(question_after_one_entry_block(false), 140);
}
