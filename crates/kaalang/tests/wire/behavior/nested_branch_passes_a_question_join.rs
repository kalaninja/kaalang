use kaalang::kaalang;

#[kaalang]
fn nested_branch_passes_a_question_join(refine: bool, finish_early: bool) -> u32 {
    #[question("Refine the value?")]
    let (nested, direct) = |refine| refine;

    #[question("Finish early?")]
    let (skip, join) = |nested, finish_early| finish_early;

    #[action("Finish without the shared step.")]
    let end = |skip| 100;

    #[action("Build the refined value.")]
    let shared = |join| 1;

    #[action("Build the direct value.")]
    let shared = |direct| 2;

    #[action("Add ten in the shared step.")]
    let end = |shared| shared + 10;

    |end| return end;
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
