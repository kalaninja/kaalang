use kaalang::kaalang;

/// One choice owning a nested pair of groups over its first three cases and a
/// separate group over its last two.
#[kaalang]
fn disjoint_and_nested_groups(source: u8) -> u8 {
    #[choice("Which source?")]
    #[case("First.")]
    #[case("Second.")]
    #[case("Third.")]
    #[case("Fourth.")]
    #[case("Fifth.")]
    let (a, b, c, d, e) = |source| match source {
        0 => (),
        1 => (),
        2 => (),
        3 => (),
        _ => (),
    };

    #[action("First value.")]
    let ab = |a| 1u8;

    #[action("Second value.")]
    let ab = |b| 2u8;

    #[action("Close the first pair.")]
    let abc = |ab| ab + 10;

    #[action("Third value.")]
    let abc = |c| 3u8;

    #[action("Finish the first three.")]
    let result = |abc| abc + 100;

    #[action("Fourth value.")]
    let de = |d| 4u8;

    #[action("Fifth value.")]
    let de = |e| 5u8;

    #[action("Finish the last two.")]
    let result = |de| de + 200;
}

#[test]
fn a_nested_group_and_a_separate_one_reach_their_own_finishes() {
    assert_eq!(disjoint_and_nested_groups(0), 111);
    assert_eq!(disjoint_and_nested_groups(1), 112);
    assert_eq!(disjoint_and_nested_groups(2), 103);
    assert_eq!(disjoint_and_nested_groups(3), 204);
    assert_eq!(disjoint_and_nested_groups(4), 205);
}
