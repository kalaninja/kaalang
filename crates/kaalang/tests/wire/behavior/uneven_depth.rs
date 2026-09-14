use kaalang::kaalang;

#[kaalang]
fn uneven_depth(condition: bool) -> u32 {
    #[question("Choose a branch depth.")]
    let (short, long) = |condition| condition;

    #[action("Build the short value.")]
    let selected = |short| 5;

    #[action("Prepare the long value.")]
    let prepared = |long| 7;

    #[action("Build the long value.")]
    let selected = |prepared| prepared + 1;

    #[action("Use the selected value.")]
    let end = |selected| selected * 2;

    |end| return end;
}

#[test]
fn branches_may_reach_convergence_at_different_depths() {
    assert_eq!(uneven_depth(true), 10);
    assert_eq!(uneven_depth(false), 16);
}
