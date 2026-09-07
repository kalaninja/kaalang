use kaalang::kaalang;

/// One choice with two disjoint convergence groups: its first two cases share
/// one continuation and its last two share another.
#[kaalang]
fn two_convergence_groups(quarter: u8) -> u32 {
    #[choice("Which quarter?")]
    #[case("The first quarter.")]
    #[case("The second quarter.")]
    #[case("The third quarter.")]
    #[case("The fourth quarter.")]
    |quarter| -> (first, second, third, fourth) {
        match quarter {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("Open the early half.")]
    |first| -> early { 1u32 };

    #[action("Open the early half the other way.")]
    |second| -> early { 2u32 };

    #[action("Close the early half.")]
    |early| -> result { early + 10 };

    #[action("Open the late half.")]
    |third| -> late { 3u32 };

    #[action("Open the late half the other way.")]
    |fourth| -> late { 4u32 };

    #[action("Close the late half.")]
    |late| -> result { late + 20 };
}

#[test]
fn each_group_reaches_its_own_continuation() {
    assert_eq!(two_convergence_groups(0), 11);
    assert_eq!(two_convergence_groups(1), 12);
    assert_eq!(two_convergence_groups(2), 23);
    assert_eq!(two_convergence_groups(3), 24);
}
