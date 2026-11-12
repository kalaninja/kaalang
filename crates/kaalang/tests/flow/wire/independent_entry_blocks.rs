use kaalang::kaa;

#[kaa]
fn independent_entry_blocks(condition: bool) -> (u32, u32) {
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

    |first, second| return (first, second);
}

#[test]
fn one_group_reaches_two_independent_entry_blocks() {
    assert_eq!(independent_entry_blocks(true), (10, 20));
    assert_eq!(independent_entry_blocks(false), (30, 40));
}
