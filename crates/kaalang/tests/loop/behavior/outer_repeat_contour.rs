use kaalang::kaalang;

#[kaalang]
fn outer_repeat_contour(mut mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Leave on the left.")]
        #[case("Advance in the middle.")]
        #[case("Advance on the right.")]
        let (leave, middle, right) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        |leave| break;

        #[action("Advance through the middle case.")]
        |middle, &mut mode| *mode = 0;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    }

    #[action("Return the mode.")]
    let end = |mode| mode;
}

/// The only break is the leftmost case, so the return climbs the right of the
/// body even though RFC 0002 §8 prefers the left contour here.
#[test]
fn the_return_takes_the_flank_the_break_leaves_clear() {
    assert_eq!(outer_repeat_contour(0), 0);
    assert_eq!(outer_repeat_contour(1), 0);
    assert_eq!(outer_repeat_contour(2), 0);
}
