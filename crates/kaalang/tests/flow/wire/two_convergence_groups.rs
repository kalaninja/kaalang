use kaalang::kaa;

/// One choice with two disjoint convergence groups: its first two cases share
/// one continuation and its last two share another.
#[kaa]
fn two_convergence_groups(quarter: u8) -> u32 {
    #[kaa::choice("Which quarter?")]
    #[kaa::case("The first quarter.")]
    #[kaa::case("The second quarter.")]
    #[kaa::case("The third quarter.")]
    #[kaa::case("The fourth quarter.")]
    let (first, second, third, fourth) = |quarter| match quarter {
        0 => (),
        1 => (),
        2 => (),
        _ => (),
    };

    #[kaa::action("Open the early half.")]
    let early = |first| 1;

    #[kaa::action("Open the early half the other way.")]
    let early = |second| 2;

    #[kaa::action("Close the early half.")]
    let end = |early| early + 10;

    #[kaa::action("Open the late half.")]
    let late = |third| 3;

    #[kaa::action("Open the late half the other way.")]
    let late = |fourth| 4;

    #[kaa::action("Close the late half.")]
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
