use kaalang::kaalang;

/// Two selections, each merged before the next one opens. The unconditional
/// finish captures both merged wires.
#[kaalang]
fn two_merges_before_one_finish(left: bool, right: bool) -> u8 {
    #[question("Left enabled?")]
    |left| -> (left_yes, left_no) { left };

    #[action("Left value.")]
    |left_yes| -> counted { 1u8 };

    #[action("Other left value.")]
    |left_no| -> counted { 2u8 };

    #[question("Right enabled?")]
    |right| -> (right_yes, right_no) { right };

    #[action("Right value.")]
    |right_yes| -> seen { 10u8 };

    #[action("Other right value.")]
    |right_no| -> seen { 20u8 };

    #[action("Finish with both merged values.")]
    |counted, seen| -> result { counted + seen };
}

#[test]
fn both_merges_reach_the_shared_finish() {
    assert_eq!(two_merges_before_one_finish(true, true), 11);
    assert_eq!(two_merges_before_one_finish(true, false), 21);
    assert_eq!(two_merges_before_one_finish(false, true), 12);
    assert_eq!(two_merges_before_one_finish(false, false), 22);
}
