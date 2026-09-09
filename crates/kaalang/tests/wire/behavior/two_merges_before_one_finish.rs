use kaalang::kaalang;

/// Two selections, each merged before the next one opens. The unconditional
/// finish captures both merged wires.
#[kaalang]
fn two_merges_before_one_finish(left: bool, right: bool) -> u8 {
    #[question("Left enabled?")]
    let (left_yes, left_no) = |left| left;

    #[action("Left value.")]
    let counted = |left_yes| 1u8;

    #[action("Other left value.")]
    let counted = |left_no| 2u8;

    #[question("Right enabled?")]
    let (right_yes, right_no) = |right| right;

    #[action("Right value.")]
    let seen = |right_yes| 10u8;

    #[action("Other right value.")]
    let seen = |right_no| 20u8;

    #[action("Finish with both merged values.")]
    let result = |counted, seen| counted + seen;
}

#[test]
fn both_merges_reach_the_shared_finish() {
    assert_eq!(two_merges_before_one_finish(true, true), 11);
    assert_eq!(two_merges_before_one_finish(true, false), 21);
    assert_eq!(two_merges_before_one_finish(false, true), 12);
    assert_eq!(two_merges_before_one_finish(false, false), 22);
}
