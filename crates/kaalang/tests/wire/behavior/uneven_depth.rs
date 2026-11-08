use kaalang::kaa;

#[kaa]
fn uneven_depth(condition: bool) -> u32 {
    #[kaa::question("Choose a branch depth.")]
    let (short, long) = |condition| condition;

    #[kaa::action("Build the short value.")]
    let selected = |short| 5;

    #[kaa::action("Prepare the long value.")]
    let prepared = |long| 7;

    #[kaa::action("Build the long value.")]
    let selected = |prepared| prepared + 1;

    #[kaa::action("Use the selected value.")]
    let end = |selected| selected * 2;

    |end| return end;
}

#[test]
fn branches_may_reach_convergence_at_different_depths() {
    assert_eq!(uneven_depth(true), 10);
    assert_eq!(uneven_depth(false), 16);
}
