use kaalang::kaalang;

#[kaalang]
fn capture_uneven(condition: bool) -> u32 {
    #[question("Choose a path depth.")]
    |condition| -> (short, long) { condition };

    #[action("Build the short result.")]
    |short| -> result { 5 };

    #[action("Prepare the long result.")]
    |long| -> prepared { 7 };

    #[action("Build the long result.")]
    |prepared| -> result { prepared + 1 };
}

#[test]
fn paths_may_reach_end_at_different_depths() {
    assert_eq!(capture_uneven(true), 5);
    assert_eq!(capture_uneven(false), 8);
}
