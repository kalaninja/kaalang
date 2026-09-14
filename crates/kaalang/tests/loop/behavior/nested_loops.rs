use kaalang::kaalang;

#[kaalang]
fn nested_loops(limit: usize) -> usize {
    #[action("Initialize the outer counter.")]
    let mut outer = || 0;

    #[cycle("Advance the outer counter to the limit.")]
    let result = |mut outer, limit| {
        #[action("Initialize the iteration counter.")]
        let mut inner = || 0;

        #[cycle("Advance the inner counter to the outer counter.")]
        |mut inner, &outer| {
            #[question("Is the iteration counter below the outer counter?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_1, leave_1) = |&inner, outer| *inner < *outer;

            |leave_1| break;

            #[action("Increment the iteration counter.")]
            |iterate_1, &mut inner| *inner += 1;
        };

        #[question("Has the outer counter reached the limit?")]
        let (done, again) = |&outer, &limit| *outer == *limit;

        |done, outer| break outer;

        #[action("Increment the outer counter.")]
        |again, &mut outer| *outer += 1;
    };

    |result| return result;
}

#[test]
fn nests_a_conditional_loop_inside_an_unconditional_loop() {
    for limit in [0, 1, 4] {
        assert_eq!(nested_loops(limit), limit);
    }
}
