use kaalang::kaalang;

#[kaalang]
fn nested_branch_passes_a_question_join(refine: bool, finish_early: bool) -> u32 {
    #[question("Refine the value?")]
    |refine| -> (nested, direct) { refine };

    #[question("Finish early?")]
    |nested, finish_early| -> (skip, join) { finish_early };

    #[action("Finish without the shared step.")]
    |skip| -> result { 100 };

    #[action("Build the refined value.")]
    |join| -> shared { 1 };

    #[action("Build the direct value.")]
    |direct| -> shared { 2 };

    #[action("Add ten in the shared step.")]
    |shared| -> result { shared + 10 };
}

#[test]
fn a_nested_branch_passes_its_question_join_without_running_it() {
    assert_eq!(nested_branch_passes_a_question_join(true, false), 11);
    assert_eq!(nested_branch_passes_a_question_join(true, true), 100);
    for finish_early in [false, true] {
        assert_eq!(
            nested_branch_passes_a_question_join(false, finish_early),
            12
        );
    }
}
