use kaalang::kaalang;

#[kaalang]
fn terminal_cases_after_repeats(mode: u8) -> u8 {
    #[cycle("Repeat until either finishing case.")]
    let result = |mut mode| {
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
        let seven_result = |seven| 7;

        |seven_result| break seven_result;

        #[action("Finish the flow with nine.")]
        let nine_result = |nine| 9;

        |nine_result| break nine_result;

        #[action("Advance through the left case.")]
        |left, &mut mode| *mode = 1;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    };

    |result| return result;
}

/// Both finishing cases lie outside the two repeating routes.
#[test]
fn finishing_cases_after_the_repeats_keep_their_results() {
    assert_eq!(terminal_cases_after_repeats(0), 7);
    assert_eq!(terminal_cases_after_repeats(1), 7);
    assert_eq!(terminal_cases_after_repeats(2), 9);
    assert_eq!(terminal_cases_after_repeats(3), 7);
}
