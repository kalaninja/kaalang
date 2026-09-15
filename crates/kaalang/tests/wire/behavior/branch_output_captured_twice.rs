use kaalang::kaalang;

/// A branch output is an ordinary wire. The first consumer enters the branch,
/// and the second reads the same value through it rather than from a second
/// connection off the branch exit.
#[kaalang]
fn branch_output_captured_twice(condition: bool) -> u8 {
    #[question("Choose the branch.")]
    let (yes, no) = |condition| condition;

    #[action("Start the yes branch.")]
    let first = |yes| 1;

    #[action("Read the same branch output again.")]
    let second = |yes| 2;

    #[action("Finish the yes branch.")]
    let end = |first, second| first + second;

    #[action("Finish the no branch.")]
    let end = |no| 0;

    |end| return end;
}

#[test]
fn both_consumers_run_inside_the_branch() {
    assert_eq!(branch_output_captured_twice(true), 3);
    assert_eq!(branch_output_captured_twice(false), 0);
}
