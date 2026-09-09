use kaalang::kaalang;

/// Three partial merges nested one inside the next: `ab` joins inside `abc`,
/// which joins inside `abcd`. Each shared body is emitted once.
#[kaalang]
fn nested_partial_merges(source: u8) -> u8 {
    #[choice("Which source?")]
    #[case("First.")]
    #[case("Second.")]
    #[case("Third.")]
    #[case("Fourth.")]
    |source| -> (a, b, c, d) {
        match source {
            0 => (),
            1 => (),
            2 => (),
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

    #[action("Close the first three.")]
    |abc| -> abcd { abc + 100 };

    #[action("Fourth value.")]
    |d| -> abcd { 4u8 };

    #[action("Finish.")]
    |abcd| -> result { abcd };
}

#[test]
fn each_case_leaves_at_the_join_that_covers_it() {
    assert_eq!(nested_partial_merges(0), 111);
    assert_eq!(nested_partial_merges(1), 112);
    assert_eq!(nested_partial_merges(2), 103);
    assert_eq!(nested_partial_merges(3), 4);
}
