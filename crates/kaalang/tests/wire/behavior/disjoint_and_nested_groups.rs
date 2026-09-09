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
    |source| -> (a, b, c, d, e) {
        match source {
            0 => (),
            1 => (),
            2 => (),
            3 => (),
            _ => (),
        }
    };

    #[action("First value.")]
    |a| -> ab { 1u8 };

    #[action("Second value.")]
    |b| -> ab { 2u8 };

    #[action("Close the first pair.")]
    |ab| -> abc { ab + 10 };

    #[action("Third value.")]
    |c| -> abc { 3u8 };

    #[action("Finish the first three.")]
    |abc| -> result { abc + 100 };

    #[action("Fourth value.")]
    |d| -> de { 4u8 };

    #[action("Fifth value.")]
    |e| -> de { 5u8 };

    #[action("Finish the last two.")]
    |de| -> result { de + 200 };
}

#[test]
fn a_nested_group_and_a_separate_one_reach_their_own_finishes() {
    assert_eq!(disjoint_and_nested_groups(0), 111);
    assert_eq!(disjoint_and_nested_groups(1), 112);
    assert_eq!(disjoint_and_nested_groups(2), 103);
    assert_eq!(disjoint_and_nested_groups(3), 204);
    assert_eq!(disjoint_and_nested_groups(4), 205);
}
