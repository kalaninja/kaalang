use kaalang::kaalang;

#[kaalang]
fn trailing_while() -> usize {
    #[action("Initialize the counters.")]
    let (mut outer, mut inner) = || (0, 0);

    loop {
        #[action("Enter the outer iteration.")]
        |&mut outer| *outer += 1;

        #[question("Is the inner counter below three?")]
        while (|&inner| *inner < 3) {
            #[action("Increment the inner counter.")]
            |&mut inner| *inner += 1;

            #[question("Has the inner counter reached three?")]
            let (done, again) = |&inner| *inner == 3;

            #[action("Return the outer counter.")]
            let end = |done, &outer| *outer;

            #[action("Finish the inner iteration.")]
            |again| {};
        }
    }
}

#[test]
fn repeats_the_inner_while_before_reentering_the_outer_loop() {
    assert_eq!(trailing_while(), 1);
}
