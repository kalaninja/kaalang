use kaalang::kaa;

/// A branch output is an ordinary wire. The first consumer enters the branch,
/// and the second reads the same value through it rather than from a second
/// connection off the branch exit.
#[kaa]
fn branch_output_captured_twice(condition: bool) -> u8 {
    #[kaa::question("Choose the branch.")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Start the yes branch.")]
    let first = |yes| 1;

    #[kaa::action("Read the same branch output again.")]
    let second = |yes| 2;

    #[kaa::action("Finish the yes branch.")]
    let end = |first, second| first + second;

    #[kaa::action("Finish the no branch.")]
    let end = |no| 0;

    |end| return end;
}

#[test]
fn both_consumers_run_inside_the_branch() {
    assert_eq!(branch_output_captured_twice(true), 3);
    assert_eq!(branch_output_captured_twice(false), 0);
}
