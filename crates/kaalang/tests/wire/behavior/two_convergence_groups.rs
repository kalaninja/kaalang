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
    let (first, second, third, fourth) = |quarter| match quarter {
        0 => (),
        1 => (),
        2 => (),
        _ => (),
    };

    #[action("Open the early half.")]
    let early = |first| 1;

    #[action("Open the early half the other way.")]
    let early = |second| 2;

    #[action("Close the early half.")]
    let end = |early| early + 10;

    #[action("Open the late half.")]
    let late = |third| 3;

    #[action("Open the late half the other way.")]
    let late = |fourth| 4;

    #[action("Close the late half.")]
    let end = |late| late + 20;

    |end| return end;
}

#[test]
fn each_group_reaches_its_own_continuation() {
    assert_eq!(two_convergence_groups(0), 11);
    assert_eq!(two_convergence_groups(1), 12);
    assert_eq!(two_convergence_groups(2), 23);
    assert_eq!(two_convergence_groups(3), 24);
}
