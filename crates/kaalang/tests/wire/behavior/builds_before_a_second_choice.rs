use kaalang::kaa;

/// The first choice's branches produce `value`, which merges before the second choice.
#[kaa]
fn builds_before_a_second_choice(first: u8, second: u8) -> u8 {
    #[kaa::choice("Which build?")]
    #[kaa::case("Build A.")]
    #[kaa::case("Build B.")]
    #[kaa::case("Build C.")]
    let (a, b, c) = |first| match first {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Build A.")]
    let value = |a| 1;

    #[kaa::action("Build B.")]
    let value = |b| 2;

    #[kaa::action("Build C.")]
    let value = |c| 3;

    #[kaa::choice("Which finish?")]
    #[kaa::case("Finish X.")]
    #[kaa::case("Finish Y.")]
    #[kaa::case("Finish Z.")]
    let (x, y, z) = |second| match second {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Finish X.")]
    let end = |x, value| value + 10;

    #[kaa::action("Finish Y.")]
    let end = |y, value| value + 20;

    #[kaa::action("Finish Z.")]
    let end = |z, value| value + 30;

    |end| return end;
}

#[test]
fn every_build_reaches_every_finish() {
    for first in 0..3 {
        for second in 0..3 {
            let expected = (first + 1) + (second + 1) * 10;
            assert_eq!(builds_before_a_second_choice(first, second), expected);
        }
    }
}
