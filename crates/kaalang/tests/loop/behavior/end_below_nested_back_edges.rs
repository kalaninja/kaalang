use kaalang::kaalang;

#[kaalang]
fn end_below_nested_back_edges(mode: u8) -> u8 {
    #[cycle("Choose an outer route until one produces a result.")]
    let result = |mut mode| {
        #[choice("Which outer route?")]
        #[case("Enter the inner loop.")]
        #[case("Finish with seven.")]
        #[case("Finish with nine.")]
        let (enter, seven, nine) = |mode| match mode {
            0 | 3 => (),
            1 => (),
            _ => (),
        };

        #[cycle("Choose an inner route until one completes.")]
        let inner_result = |enter, &mut mode| {
            #[choice("Which inner route?")]
            #[case("Repeat the inner loop.")]
            #[case("Repeat the outer loop.")]
            #[case("Finish with eleven.")]
            let (repeat, leave, eleven) = |&mode| match **mode {
                0 | 3 => (),
                1 => (),
                _ => (),
            };

            #[action("Advance to the next route.")]
            |repeat, &mut mode| **mode += 1;

            #[action("Continue with the outer cycle.")]
            let outcome = |leave| None;

            #[action("Produce eleven.")]
            let outcome = |eleven| Some(11);

            |outcome| break outcome;
        };

        #[question("Did the inner cycle produce a result?")]
        let (_repeat_outer, finish_inner) = |&inner_result| inner_result.is_none();

        #[action("Extract the inner result.")]
        let selected = |finish_inner, inner_result| {
            inner_result.expect("the completing inner route has a result")
        };

        #[action("Produce seven.")]
        let selected = |seven| 7;

        #[action("Produce nine.")]
        let selected = |nine| 9;

        |selected| break selected;
    };

    |result| return result;
}

#[test]
fn nested_repeats_and_terminal_routes_keep_their_results() {
    assert_eq!(end_below_nested_back_edges(0), 7);
    assert_eq!(end_below_nested_back_edges(1), 7);
    assert_eq!(end_below_nested_back_edges(2), 9);
    assert_eq!(end_below_nested_back_edges(3), 11);
}
