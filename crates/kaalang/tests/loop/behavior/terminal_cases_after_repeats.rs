use kaalang::kaalang;

#[kaalang]
fn terminal_cases_after_repeats(mut mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Advance on the left.")]
        #[case("Advance on the right.")]
        #[case("Finish with seven.")]
        #[case("Finish with nine.")]
        let (left, right, seven, nine) = |mode| match mode {
            0 => (),
            3.. => (),
            1 => (),
            _ => (),
        };

        #[action("Finish the flow with seven.")]
        let end = |seven| 7;

        #[action("Finish the flow with nine.")]
        let end = |nine| 9;

        #[action("Advance through the left case.")]
        |left, &mut mode| *mode = 1;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    }
}

/// Both finishing cases lie outside the two repeating routes.
#[test]
fn finishing_cases_after_the_repeats_keep_their_results() {
    assert_eq!(terminal_cases_after_repeats(0), 7);
    assert_eq!(terminal_cases_after_repeats(1), 7);
    assert_eq!(terminal_cases_after_repeats(2), 9);
    assert_eq!(terminal_cases_after_repeats(3), 7);
}
