use kaalang::kaalang;

#[kaalang]
fn question_after_one_entry_block(condition: bool) -> u32 {
    #[question("Choose the pair.")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes pair.")]
    let (left, right) = |yes| (1, 2);

    #[action("Build the no pair.")]
    let (left, right) = |no| (3, 4);

    #[action("Use the left value.")]
    let first = |left| left * 10;

    #[action("Use the right value.")]
    let second = |right| right * 10;

    #[question("Is the first value large?")]
    let (large, small) = |first| first > 10;

    #[action("Combine a large first value with the second.")]
    let result = |large, second| second + 100;

    #[action("Combine a small first value with the second.")]
    let result = |small, second| second;
}

#[test]
fn a_question_after_one_entry_block_selects_consumers_of_the_other() {
    assert_eq!(question_after_one_entry_block(true), 20);
    assert_eq!(question_after_one_entry_block(false), 140);
}
