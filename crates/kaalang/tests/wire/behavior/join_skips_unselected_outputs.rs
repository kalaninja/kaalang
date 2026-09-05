use kaalang::kaalang;

#[kaalang]
fn join_skips_unselected_outputs(outer: bool, inner: bool) -> u8 {
    #[question("Choose an outer branch.")]
    |outer| -> (left, right) { outer };

    #[question("Select an unused wire.")]
    |&left, inner| -> (_unused, _other) { inner };

    #[action("Produce the left value.")]
    |&left| -> selected { 1u8 };

    #[action("Produce the right value and an unused alternative.")]
    |right| -> (_unused, selected) { ((), 2u8) };

    #[action("Use the selected value.")]
    |selected| -> result { selected };

    #[end]
    |result| {};
}

#[test]
fn a_join_carries_only_outputs_available_in_every_execution() {
    for inner in [false, true] {
        assert_eq!(join_skips_unselected_outputs(true, inner), 1);
        assert_eq!(join_skips_unselected_outputs(false, inner), 2);
    }
}
