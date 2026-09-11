use kaalang::kaalang;

#[kaalang]
fn terminal_case_between_repeats(mut mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Advance on the left.")]
        #[case("Finish in the middle.")]
        #[case("Advance on the right.")]
        let (left, middle, right) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Finish from the middle case.")]
        let end = |middle| 7;

        #[action("Advance through the left case.")]
        |left, &mut mode| *mode = 1;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    }
}

/// The middle case finishes the flow from inside the body. Nothing forces end
/// below the iteration tail, so the tail sinks past it and both flanks stay
/// clear.
#[test]
fn a_finishing_case_does_not_block_the_contour() {
    assert_eq!(terminal_case_between_repeats(0), 7);
    assert_eq!(terminal_case_between_repeats(1), 7);
    assert_eq!(terminal_case_between_repeats(2), 7);
}
