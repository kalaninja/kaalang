use kaalang::kaalang;

#[kaalang]
fn end_below_nested_returns(mut mode: u8) -> u8 {
    loop {
        #[choice("Which outer route?")]
        #[case("Enter the inner loop.")]
        #[case("Finish with seven.")]
        #[case("Finish with nine.")]
        let (enter, seven, nine) = |mode| match mode {
            0 | 3 => (),
            1 => (),
            _ => (),
        };

        |enter| loop {
            #[choice("Which inner route?")]
            #[case("Repeat the inner loop.")]
            #[case("Repeat the outer loop.")]
            #[case("Finish with eleven.")]
            let (repeat, leave, eleven) = |mode| match mode {
                0 | 3 => (),
                1 => (),
                _ => (),
            };

            #[action("Advance to the next route.")]
            |repeat, &mut mode| *mode += 1;

            |leave| break;

            #[action("Return eleven.")]
            let end = |eleven| 11;
        };

        #[action("Return seven.")]
        let end = |seven| 7;

        #[action("Return nine.")]
        let end = |nine| 9;
    }
}

#[test]
fn nested_repeats_and_terminal_routes_keep_their_results() {
    assert_eq!(end_below_nested_returns(0), 7);
    assert_eq!(end_below_nested_returns(1), 7);
    assert_eq!(end_below_nested_returns(2), 9);
    assert_eq!(end_below_nested_returns(3), 11);
}
