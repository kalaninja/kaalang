use kaalang::kaa;

#[kaa]
fn end_below_nested_back_edges(mut mode: u8) -> u8 {
    #[kaa::cycle("Choose an outer route until one produces a result.")]
    let selected = loop {
        #[kaa::choice("Which outer route?")]
        #[kaa::case("Enter the inner cycle.")]
        #[kaa::case("Finish with seven.")]
        #[kaa::case("Finish with nine.")]
        let (enter, seven, nine) = |mode| match mode {
            0 | 3 => (),
            1 => (),
            _ => (),
        };

        #[kaa::cycle("Choose an inner route until one completes.")]
        let inner_result = |enter| loop {
            #[kaa::choice("Which inner route?")]
            #[kaa::case("Repeat the inner cycle.")]
            #[kaa::case("Repeat the outer cycle.")]
            #[kaa::case("Finish with eleven.")]
            let (repeat, leave, eleven) = |&mode| match *mode {
                0 | 3 => (),
                1 => (),
                _ => (),
            };

            #[kaa::action("Advance to the next route.")]
            |repeat, &mut mode| *mode += 1;

            #[kaa::action("Continue with the outer cycle.")]
            let inner_result = |leave| None;

            #[kaa::action("Produce eleven.")]
            let inner_result = |eleven| Some(11);

            |repeat| continue;
        };

        #[kaa::question("Did the inner cycle produce a result?")]
        let (repeat_outer, finish_inner) = |&inner_result| inner_result.is_none();

        #[kaa::action("Extract the inner result.")]
        let selected = |finish_inner, inner_result| {
            inner_result.expect("the completing inner route has a result")
        };

        #[kaa::action("Produce seven.")]
        let selected = |seven| 7;

        #[kaa::action("Produce nine.")]
        let selected = |nine| 9;

        |repeat_outer| continue;
    };

    |selected| return selected;
}

#[test]
fn nested_repeats_and_terminal_routes_keep_their_results() {
    assert_eq!(end_below_nested_back_edges(0), 7);
    assert_eq!(end_below_nested_back_edges(1), 7);
    assert_eq!(end_below_nested_back_edges(2), 9);
    assert_eq!(end_below_nested_back_edges(3), 11);
}
