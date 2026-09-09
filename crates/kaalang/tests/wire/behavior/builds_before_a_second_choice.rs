use kaalang::kaalang;

/// The corrected order for two selections: the first choice's builds merge into
/// `value` before the second choice opens its own branches.
#[kaalang]
fn builds_before_a_second_choice(first: u8, second: u8) -> u8 {
    #[choice("Which build?")]
    #[case("Build A.")]
    #[case("Build B.")]
    #[case("Build C.")]
    |first| -> (a, b, c) {
        match first {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build A.")]
    |a| -> value { 1u8 };

    #[action("Build B.")]
    |b| -> value { 2u8 };

    #[action("Build C.")]
    |c| -> value { 3u8 };

    #[choice("Which finish?")]
    #[case("Finish X.")]
    #[case("Finish Y.")]
    #[case("Finish Z.")]
    |second| -> (x, y, z) {
        match second {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Finish X.")]
    |x, value| -> result { value + 10 };

    #[action("Finish Y.")]
    |y, value| -> result { value + 20 };

    #[action("Finish Z.")]
    |z, value| -> result { value + 30 };
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
