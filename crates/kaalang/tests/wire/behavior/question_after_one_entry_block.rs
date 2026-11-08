use kaalang::kaa;

#[kaa]
fn question_after_one_entry_block(condition: bool) -> u32 {
    #[kaa::question("Choose the pair.")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Build the yes pair.")]
    let (left, right) = |yes| (1, 2);

    #[kaa::action("Build the no pair.")]
    let (left, right) = |no| (3, 4);

    #[kaa::action("Use the left value.")]
    let first = |left| left * 10;

    #[kaa::action("Use the right value.")]
    let second = |right| right * 10;

    #[kaa::question("Is the first value large?")]
    let (large, small) = |first| first > 10;

    #[kaa::action("Combine a large first value with the second.")]
    let end = |large, second| second + 100;

    #[kaa::action("Combine a small first value with the second.")]
    let end = |small, second| second;

    |end| return end;
}

#[test]
fn a_question_after_one_entry_block_selects_consumers_of_the_other() {
    assert_eq!(question_after_one_entry_block(true), 20);
    assert_eq!(question_after_one_entry_block(false), 140);
}
