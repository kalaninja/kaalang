use kaalang::kaalang;

#[kaalang]
fn trailing_inner_loop() -> usize {
    #[action("Initialize the counters.")]
    let (mut outer, mut inner) = || (0, 0);

    loop {
        #[action("Enter the outer iteration.")]
        |&mut outer| *outer += 1;

        |&inner| loop {
            #[question("Is the inner counter below three?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_1, leave_1) = |&inner| *inner < 3;

            |leave_1| break;

            #[action("Increment the inner counter.")]
            let incremented = |iterate_1, &mut inner| *inner += 1;

            #[question("Has the inner counter reached three?")]
            let (done, again) = |incremented, &inner| *inner == 3;

            #[action("Return the outer counter.")]
            let end = |done, &outer| *outer;

            #[action("Finish the inner iteration.")]
            |again| {};
        };
    }
}

#[test]
fn repeats_the_inner_loop_before_reentering_the_outer_loop() {
    assert_eq!(trailing_inner_loop(), 1);
}
