use kaalang::kaalang;

#[kaalang]
fn uneven_depth(condition: bool) -> u32 {
    #[question("Choose a path depth.")]
    |condition| -> (short, long) { condition };

    #[action("Build the short value.")]
    |short| -> selected { 5 };

    #[action("Prepare the long value.")]
    |long| -> prepared { 7 };

    #[action("Build the long value.")]
    |prepared| -> selected { prepared + 1 };

    #[action("Use the selected value.")]
    |selected| -> result { selected * 2 };

    #[end]
    |result| {};
}

#[test]
fn branches_may_reach_convergence_at_different_depths() {
    assert_eq!(uneven_depth(true), 10);
    assert_eq!(uneven_depth(false), 16);
}
