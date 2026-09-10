use kaalang::kaalang;

#[kaalang]
fn converged_selection_meets_a_branch(left: bool, right: bool) -> u8 {
    #[question("Right enabled?")]
    let (b, right_no) = |right| right;

    #[action("Provide the right value.")]
    let right_value = |b| Some(1);

    #[action("Provide no right value.")]
    let right_value = |right_no| None;

    #[question("Left enabled?")]
    let (a, left_no) = |left| left;

    #[action("Work with the right value.")]
    let end = |a, right_value| right_value.unwrap_or(0) + 10;

    #[action("Skip the work.")]
    let end = |left_no, right_value| right_value.map_or(0, |_| 0);
}

#[test]
fn only_the_left_question_decides_the_work() {
    assert_eq!(converged_selection_meets_a_branch(true, true), 11);
    assert_eq!(converged_selection_meets_a_branch(true, false), 10);
    assert_eq!(converged_selection_meets_a_branch(false, true), 0);
    assert_eq!(converged_selection_meets_a_branch(false, false), 0);
}
