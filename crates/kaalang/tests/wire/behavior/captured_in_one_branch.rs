use kaalang::kaa;

#[kaa]
fn captured_in_one_branch(condition: bool, extra: u32) -> u32 {
    #[kaa::question("Use the extra value?")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Add a fixed amount to the extra value.")]
    let end = |yes, extra| extra + 5;

    #[kaa::action("Ignore the extra value.")]
    let end = |no| 0;

    |end| return end;
}

#[test]
fn a_producer_captured_in_one_execution_is_valid() {
    assert_eq!(captured_in_one_branch(true, 3), 8);
    assert_eq!(captured_in_one_branch(false, 3), 0);
}
