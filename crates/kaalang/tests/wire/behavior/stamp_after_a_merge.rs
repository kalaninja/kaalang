use kaalang::kaalang;

/// An action with no inputs after a completed merge, capturing nothing from it.
#[kaalang]
fn stamp_after_a_merge(flag: bool) -> u8 {
    #[question("Which value?")]
    let (yes, no) = |flag| flag;

    #[action("Build the yes value.")]
    let value = |yes| 1;

    #[action("Build the no value.")]
    let value = |no| 2;

    #[action("Stamp.")]
    let stamp = || 10;

    #[action("Finish.")]
    let end = |value, stamp| value + stamp;
}

#[test]
fn a_no_input_action_runs_on_the_common_path_after_a_merge() {
    assert_eq!(stamp_after_a_merge(true), 11);
    assert_eq!(stamp_after_a_merge(false), 12);
}
