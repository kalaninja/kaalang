use kaalang::kaalang;

#[kaalang]
fn trailing_inner_loop() -> usize {
    #[action("Initialize the counters.")]
    let (mut outer, mut inner) = || (0, 0);

    #[cycle("Repeat outer iterations until the inner cycle finishes.")]
    let result = |mut outer, mut inner| {
        #[action("Enter the outer iteration.")]
        |&mut outer| *outer += 1;

        #[cycle("Advance the inner counter or request another outer pass.")]
        let inner_result = |&mut inner, &outer| {
            #[question("Is the inner counter below three?")]
            #[no("NO")]
            #[yes("YES")]
            let (leave_1, iterate_1) = |&inner| **inner < 3;

            #[action("Request another outer pass.")]
            let outcome = |leave_1| None;

            #[action("Increment the inner counter.")]
            let incremented = |iterate_1, &mut inner| **inner += 1;

            #[question("Has the inner counter reached three?")]
            let (done, again) = |incremented, &inner| **inner == 3;

            #[action("Produce the outer counter.")]
            let outcome = |done, outer| Some(*outer);

            |outcome| break outcome;

            #[action("Finish the inner iteration.")]
            |again| {};
        };

        #[question("Did the inner cycle finish the flow?")]
        let (done, again) = |&inner_result| inner_result.is_some();

        #[action("Extract the result.")]
        let result = |done, inner_result| inner_result.expect("the completed cycle has a result");

        |result| break result;

        #[action("Finish the outer iteration.")]
        |again| {};
    };

    |result| return result;
}

#[test]
fn repeats_the_inner_loop_before_reentering_the_outer_loop() {
    assert_eq!(trailing_inner_loop(), 1);
}
