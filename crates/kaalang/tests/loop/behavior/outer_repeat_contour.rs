use kaalang::kaalang;

#[kaalang]
fn outer_repeat_contour(mut mode: u8) -> u8 {
    #[cycle("Advance until the left route leaves.")]
    let leave = {
        #[choice("Which route?")]
        #[case("Leave on the left.")]
        #[case("Advance in the middle.")]
        #[case("Advance on the right.")]
        let (leave, middle, right) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Advance through the middle case.")]
        let advanced = |middle, &mut mode| *mode = 0;

        #[action("Advance through the right case.")]
        let advanced = |right, &mut mode| *mode = 1;

        |advanced| continue;
    };

    |leave, mode| return mode;
}

/// The only completing case is the leftmost, so the iteration back edge climbs the
/// right of the body although the left contour is preferred here.
#[test]
fn the_back_edge_takes_the_flank_the_exit_leaves_clear() {
    assert_eq!(outer_repeat_contour(0), 0);
    assert_eq!(outer_repeat_contour(1), 0);
    assert_eq!(outer_repeat_contour(2), 0);
}
