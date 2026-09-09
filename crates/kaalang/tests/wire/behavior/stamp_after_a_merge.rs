use kaalang::kaalang;

/// An action with no inputs after a completed merge, capturing nothing from it.
#[kaalang]
fn stamp_after_a_merge(flag: bool) -> u8 {
    #[question("Which value?")]
    |flag| -> (yes, no) { flag };

    #[action("Build the yes value.")]
    |yes| -> value { 1u8 };

    #[action("Build the no value.")]
    |no| -> value { 2u8 };

    #[action("Stamp.")]
    || -> stamp { 10u8 };

    #[action("Finish.")]
    |value, stamp| -> result { value + stamp };
}

#[test]
fn a_no_input_action_runs_on_the_common_path_after_a_merge() {
    assert_eq!(stamp_after_a_merge(true), 11);
    assert_eq!(stamp_after_a_merge(false), 12);
}
