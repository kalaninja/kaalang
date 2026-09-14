use kaalang::kaalang;

#[kaalang]
fn outer_repeat_contour(mode: u8) -> u8 {
    #[cycle("Advance until the left route leaves.")]
    let result = |mut mode| {
        #[choice("Which route?")]
        #[case("Leave on the left.")]
        #[case("Advance in the middle.")]
        #[case("Advance on the right.")]
        let (leave, middle, right) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        |leave, mode| break mode;

        #[action("Advance through the middle case.")]
        |middle, &mut mode| *mode = 0;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    };

    |result| return result;
}

/// The only break is the leftmost case, so the iteration back edge climbs the
/// right of the body even though RFC 0002 §8 prefers the left contour here.
#[test]
fn the_back_edge_takes_the_flank_the_break_leaves_clear() {
    assert_eq!(outer_repeat_contour(0), 0);
    assert_eq!(outer_repeat_contour(1), 0);
    assert_eq!(outer_repeat_contour(2), 0);
}
