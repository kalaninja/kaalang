use kaalang::kaalang;

/// Common work after a selection needs the branches to merge first. Both
/// branches provide a unit-valued `ready` wire, and the transform follows it.
#[kaalang]
fn branch_log_with_merge(condition: bool, value: String) -> usize {
    #[question("Log the value?")]
    let (yes, no) = |condition| condition;

    #[action("Log.")]
    let ready = |yes, &value| {
        assert_eq!(value, "abc");
    };

    #[action("Continue without logging.")]
    let ready = |no| {};

    #[action("Transform.")]
    let end = |ready, value| value.len();
}

#[test]
fn the_transform_runs_once_after_the_selected_branch() {
    assert_eq!(branch_log_with_merge(true, String::from("abc")), 3);
    assert_eq!(branch_log_with_merge(false, String::from("abc")), 3);
}
