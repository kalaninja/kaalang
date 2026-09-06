use kaalang::kaalang;

#[kaalang]
fn converged_selection_meets_a_branch(left: bool, right: bool) -> u8 {
    #[question("Left enabled?")]
    |left| -> (a, left_no) { left };

    #[question("Right enabled?")]
    |right| -> (b, right_no) { right };

    #[action("Provide the right value.")]
    |b| -> right_value { Some(1u8) };

    #[action("Provide no right value.")]
    |right_no| -> right_value { None };

    #[action("Work with the right value.")]
    |a, right_value| -> result { right_value.unwrap_or(0) + 10 };

    #[action("Skip the work.")]
    |left_no, right_value| -> result { right_value.map_or(0, |_| 0) };
}

#[test]
fn only_the_left_question_decides_the_work() {
    assert_eq!(converged_selection_meets_a_branch(true, true), 11);
    assert_eq!(converged_selection_meets_a_branch(true, false), 10);
    assert_eq!(converged_selection_meets_a_branch(false, true), 0);
    assert_eq!(converged_selection_meets_a_branch(false, false), 0);
}
