use kaalang::kaalang;

#[kaalang]
fn join_skips_unselected_outputs(outer: bool, inner: bool) -> u8 {
    #[question("Choose an outer branch.")]
    |outer| -> (left, right) { outer };

    #[action("Prepare the left branch.")]
    |left| -> (probe, left_value) { ((), 1u8) };

    #[question("Refine the left branch.")]
    |probe, inner| -> (near, far) { inner };

    #[action("Produce the near left value and an unused alternative.")]
    |near, &left_value| -> (_unused, selected) { ((), *left_value) };

    #[action("Produce the far left value.")]
    |far, &left_value| -> selected { *left_value };

    #[action("Produce the right value and an unused alternative.")]
    |right| -> (_unused, selected) { ((), 2u8) };

    #[action("Use the selected value.")]
    |selected| -> result { selected };
}

#[test]
fn a_join_carries_only_outputs_available_in_every_execution() {
    for inner in [false, true] {
        assert_eq!(join_skips_unselected_outputs(true, inner), 1);
        assert_eq!(join_skips_unselected_outputs(false, inner), 2);
    }
}
