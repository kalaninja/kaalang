use kaalang::kaa;

/// An early branch can rejoin the main path below a side branch that occupies
/// the column next to it. The first question's NO route and the last question's
/// NO route converge on one wire, with the odd value's action standing between
/// them beside the main line.
#[kaa]
fn early_branch_rejoins_below_a_side_branch(value: u8) -> u8 {
    #[kaa::question("Is the value too large?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (large, kept) = |value| value > 9;

    #[kaa::question("Is the large value even?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (even, odd) = |large, value| value.is_multiple_of(2);

    #[kaa::action("Halve the even value.")]
    let halved = |even, value| value / 2;

    #[kaa::action("Drop the odd value's last bit, then halve it.")]
    let halved = |odd, value| (value - 1) / 2;

    #[kaa::question("Is the halved value small enough?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (shrunk, kept) = |halved| halved < 10;

    #[kaa::action("Use the halved value.")]
    let end = |shrunk, halved| halved;

    #[kaa::action("Use the value as it is.")]
    let end = |kept, value| value;

    |end| return end;
}

#[test]
fn only_a_large_value_with_a_small_half_is_halved() {
    assert_eq!(early_branch_rejoins_below_a_side_branch(4), 4);
    assert_eq!(early_branch_rejoins_below_a_side_branch(12), 6);
    assert_eq!(early_branch_rejoins_below_a_side_branch(13), 6);
    assert_eq!(early_branch_rejoins_below_a_side_branch(40), 40);
}
