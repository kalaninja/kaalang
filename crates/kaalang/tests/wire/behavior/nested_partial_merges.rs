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
    let (a, b, c, d) = |source| match source {
        0 => (),
        1 => (),
        2 => (),
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

    #[action("Close the first three.")]
    let abcd = |abc| abc + 100;

    #[action("Fourth value.")]
    let abcd = |d| 4u8;

    #[action("Finish.")]
    let end = |abcd| abcd;
}

#[test]
fn each_case_leaves_at_the_join_that_covers_it() {
    assert_eq!(nested_partial_merges(0), 111);
    assert_eq!(nested_partial_merges(1), 112);
    assert_eq!(nested_partial_merges(2), 103);
    assert_eq!(nested_partial_merges(3), 4);
}
