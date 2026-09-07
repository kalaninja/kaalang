use kaalang::kaalang;

/// RFC 0001 §7: a consumer can capture several merged wires. Two independent
/// questions each converge their own wire, and one block captures both, so the
/// two junctions land on the main line in turn rather than beside each other.
#[kaalang]
fn two_merges_reach_one_consumer(scale: bool, offset: bool) -> u32 {
    #[question("Scale the value?")]
    |scale| -> (scaled, plain) { scale };

    #[action("Take the scaled factor.")]
    |scaled| -> factor { 10u32 };

    #[action("Take the plain factor.")]
    |plain| -> factor { 1u32 };

    #[question("Offset the value?")]
    |offset| -> (shifted, centred) { offset };

    #[action("Take the shifted base.")]
    |shifted| -> base { 5u32 };

    #[action("Take the centred base.")]
    |centred| -> base { 0u32 };

    #[action("Combine both merged values.")]
    |factor, base| -> result { factor + base };
}

#[test]
fn one_block_captures_two_independently_merged_wires() {
    assert_eq!(two_merges_reach_one_consumer(true, true), 15);
    assert_eq!(two_merges_reach_one_consumer(true, false), 10);
    assert_eq!(two_merges_reach_one_consumer(false, true), 6);
    assert_eq!(two_merges_reach_one_consumer(false, false), 1);
}
