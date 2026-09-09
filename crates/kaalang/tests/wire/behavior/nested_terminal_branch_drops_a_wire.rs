use kaalang::kaalang;

#[kaalang]
fn nested_terminal_branch_drops_a_wire(outer: bool, inner: bool) -> u32 {
    #[question("Take the nested branch?")]
    let (nested, direct) = |outer, &inner| outer;

    #[action("Prepare the nested values.")]
    let (probe, extra) = |nested| (1, 10);

    #[question("Finish early?")]
    let (early, late) = |probe, inner| inner;

    #[action("Produce the early result without the extra value.")]
    let result = |early| 1;

    #[action("Combine the late result with the extra value.")]
    let result = |late, extra| 2 + extra;

    #[action("Produce the direct result.")]
    let result = |direct| 3;
}

#[test]
fn a_nested_question_may_finish_the_branch_before_the_extra_value_is_used() {
    assert_eq!(nested_terminal_branch_drops_a_wire(true, true), 1);
    assert_eq!(nested_terminal_branch_drops_a_wire(true, false), 12);
    assert_eq!(nested_terminal_branch_drops_a_wire(false, true), 3);
}
