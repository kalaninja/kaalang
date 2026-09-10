use kaalang::kaalang;

/// The corrected order for two selections: the first choice's builds merge into
/// `value` before the second choice opens its own branches.
#[kaalang]
fn builds_before_a_second_choice(first: u8, second: u8) -> u8 {
    #[choice("Which build?")]
    #[case("Build A.")]
    #[case("Build B.")]
    #[case("Build C.")]
    let (a, b, c) = |first| match first {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Build A.")]
    let value = |a| 1u8;

    #[action("Build B.")]
    let value = |b| 2u8;

    #[action("Build C.")]
    let value = |c| 3u8;

    #[choice("Which finish?")]
    #[case("Finish X.")]
    #[case("Finish Y.")]
    #[case("Finish Z.")]
    let (x, y, z) = |second| match second {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Finish X.")]
    let end = |x, value| value + 10;

    #[action("Finish Y.")]
    let end = |y, value| value + 20;

    #[action("Finish Z.")]
    let end = |z, value| value + 30;
}

#[test]
fn every_build_reaches_every_finish() {
    for first in 0..3u8 {
        for second in 0..3u8 {
            let expected = (first + 1) + (second + 1) * 10;
            assert_eq!(builds_before_a_second_choice(first, second), expected);
        }
    }
}
