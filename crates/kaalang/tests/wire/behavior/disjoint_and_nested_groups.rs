use kaalang::kaa;

/// One choice owning a nested pair of groups over its first three cases and a
/// separate group over its last two.
#[kaa]
fn disjoint_and_nested_groups(source: u8) -> u8 {
    #[kaa::choice("Which source?")]
    #[kaa::case("First.")]
    #[kaa::case("Second.")]
    #[kaa::case("Third.")]
    #[kaa::case("Fourth.")]
    #[kaa::case("Fifth.")]
    let (a, b, c, d, e) = |source| match source {
        0 => (),
        1 => (),
        2 => (),
        3 => (),
        _ => (),
    };

    #[kaa::action("First value.")]
    let ab = |a| 1;

    #[kaa::action("Second value.")]
    let ab = |b| 2;

    #[kaa::action("Close the first pair.")]
    let abc = |ab| ab + 10;

    #[kaa::action("Third value.")]
    let abc = |c| 3;

    #[kaa::action("Finish the first three.")]
    let end = |abc| abc + 100;

    #[kaa::action("Fourth value.")]
    let de = |d| 4;

    #[kaa::action("Fifth value.")]
    let de = |e| 5;

    #[kaa::action("Finish the last two.")]
    let end = |de| de + 200;

    |end| return end;
}

#[test]
fn a_nested_group_and_a_separate_one_reach_their_own_finishes() {
    assert_eq!(disjoint_and_nested_groups(0), 111);
    assert_eq!(disjoint_and_nested_groups(1), 112);
    assert_eq!(disjoint_and_nested_groups(2), 103);
    assert_eq!(disjoint_and_nested_groups(3), 204);
    assert_eq!(disjoint_and_nested_groups(4), 205);
}
