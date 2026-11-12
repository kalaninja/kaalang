use kaalang::kaa;

#[kaa]
fn terminal_cases_after_repeats(mut mode: u8) -> u8 {
    #[kaa::cycle("Repeat until either finishing case.")]
    let selected = loop {
        #[kaa::choice("Which route?")]
        #[kaa::case("Advance on the left.")]
        #[kaa::case("Advance on the right.")]
        #[kaa::case("Finish with seven.")]
        #[kaa::case("Finish with nine.")]
        let (left, right, seven, nine) = |mode| match mode {
            0 => (),
            3.. => (),
            1 => (),
            _ => (),
        };

        #[kaa::action("Finish the flow with seven.")]
        let selected = |seven| 7;

        #[kaa::action("Finish the flow with nine.")]
        let selected = |nine| 9;

        #[kaa::action("Advance through the left case.")]
        let advanced = |left, &mut mode| *mode = 1;

        #[kaa::action("Advance through the right case.")]
        let advanced = |right, &mut mode| *mode = 1;

        |advanced| continue;
    };

    |selected| return selected;
}

/// Both finishing cases lie outside the two repeating routes.
#[test]
fn finishing_cases_after_the_repeats_keep_their_results() {
    assert_eq!(terminal_cases_after_repeats(0), 7);
    assert_eq!(terminal_cases_after_repeats(1), 7);
    assert_eq!(terminal_cases_after_repeats(2), 9);
    assert_eq!(terminal_cases_after_repeats(3), 7);
}
