use kaalang::kaalang;

/// An action with no inputs runs on the common path where it is written. Only
/// one branch captures its output, which does not place the action in that
/// branch.
#[kaalang]
fn stamp_before_a_selection(flag: bool) -> u64 {
    #[action("Stamp.")]
    let stamp = || 7u64;

    #[question("Which way?")]
    let (yes, no) = |flag| flag;

    #[action("Finish with the stamp.")]
    let end = |yes, stamp| stamp;

    #[action("Finish without it.")]
    let end = |no| 0;
}

#[test]
fn a_no_input_action_runs_above_the_selection_it_precedes() {
    assert_eq!(stamp_before_a_selection(true), 7);
    assert_eq!(stamp_before_a_selection(false), 0);
}
