use kaalang::kaa;

#[kaa]
fn join_skips_unselected_outputs(outer: bool, inner: bool) -> u8 {
    #[kaa::question("Choose an outer branch.")]
    let (left, right) = |outer| outer;

    #[kaa::action("Prepare the left branch.")]
    let (probe, left_value) = |left| ((), 1);

    #[kaa::question("Take the far left branch?")]
    let (far, near) = |probe, inner| !inner;

    #[kaa::action("Produce the near left value and an unused alternative.")]
    let (_unused, selected) = |near, &left_value| ((), *left_value);

    #[kaa::action("Produce the far left value.")]
    let selected = |far, &left_value| *left_value;

    #[kaa::action("Produce the right value and an unused alternative.")]
    let (_unused, selected) = |right| ((), 2);

    |selected| return selected;
}

#[test]
fn a_join_carries_only_outputs_available_in_every_execution() {
    for inner in [false, true] {
        assert_eq!(join_skips_unselected_outputs(true, inner), 1);
        assert_eq!(join_skips_unselected_outputs(false, inner), 2);
    }
}
