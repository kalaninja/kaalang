use kaalang::kaalang;

#[kaalang]
fn nested_loops(limit: usize) -> usize {
    #[action("Initialize the outer counter.")]
    let mut outer = || 0;

    #[cycle("Advance the outer counter to the limit.")]
    let done = {
        #[action("Initialize the iteration counter.")]
        let mut inner = || 0;

        #[cycle("Advance the inner counter to the outer counter.")]
        let leave_1 = {
            #[question("Is the iteration counter below the outer counter?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_1, leave_1) = |&inner, &outer| *inner < *outer;

            #[action("Increment the iteration counter.")]
            |iterate_1, &mut inner| *inner += 1;

            |iterate_1| continue;
        };

        #[question("Has the outer counter reached the limit?")]
        let (done, again) = |leave_1, &outer, &limit| *outer == *limit;

        #[action("Increment the outer counter.")]
        |again, &mut outer| *outer += 1;

        |again| continue;
    };

    |done, outer| return outer;
}

#[test]
fn nests_a_conditional_loop_inside_an_unconditional_loop() {
    for limit in [0, 1, 4] {
        assert_eq!(nested_loops(limit), limit);
    }
}
