use kaalang::kaa;

#[kaa]
fn trailing_inner_cycle() -> usize {
    #[kaa::action("Initialize the counters.")]
    let (mut outer, mut inner) = || (0, 0);

    #[kaa::cycle("Repeat outer iterations until the inner cycle finishes.")]
    let result = loop {
        #[kaa::action("Enter the outer iteration.")]
        |&mut outer| *outer += 1;

        #[kaa::cycle("Advance the inner counter or request another outer pass.")]
        let inner_result = loop {
            #[kaa::question("Is the inner counter below three?")]
            #[kaa::no("NO")]
            #[kaa::yes("YES")]
            let (leave_1, iterate_1) = |&inner| *inner < 3;

            #[kaa::action("Request another outer pass.")]
            let inner_result = |leave_1| None;

            #[kaa::action("Increment the inner counter.")]
            let incremented = |iterate_1, &mut inner| *inner += 1;

            #[kaa::question("Has the inner counter reached three?")]
            let (done, again) = |incremented, &inner| *inner == 3;

            #[kaa::action("Produce the outer counter.")]
            let inner_result = |done, outer| Some(outer);

            #[kaa::action("Finish the inner iteration.")]
            |again| {};

            |again| continue;
        };

        #[kaa::question("Did the inner cycle finish the flow?")]
        let (done, again) = |&inner_result| inner_result.is_some();

        #[kaa::action("Extract the result.")]
        let result = |done, inner_result| inner_result.expect("the completed cycle has a result");

        #[kaa::action("Finish the outer iteration.")]
        |again| {};

        |again| continue;
    };

    |result| return result;
}

#[test]
fn repeats_the_inner_cycle_before_reentering_the_outer_cycle() {
    assert_eq!(trailing_inner_cycle(), 1);
}
