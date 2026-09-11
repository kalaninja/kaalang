use kaalang::kaalang;

#[kaalang]
fn nested_loops(limit: usize) -> usize {
    #[action("Initialize the outer counter.")]
    let mut outer = || 0;

    loop {
        #[action("Initialize the iteration counter.")]
        let mut inner = || 0;

        |&inner, &outer| loop {
            #[question("Is the iteration counter below the outer counter?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_1, leave_1) = |&inner, &outer| *inner < *outer;

            |leave_1| break;

            #[action("Increment the iteration counter.")]
            |iterate_1, &mut inner| *inner += 1;
        };

        #[question("Has the outer counter reached the limit?")]
        let (done, again) = |&outer, &limit| *outer == *limit;

        #[action("Return the counter.")]
        let end = |done, &outer| *outer;

        #[action("Increment the outer counter.")]
        |again, &mut outer| *outer += 1;
    }
}

#[test]
fn nests_a_conditional_loop_inside_an_unconditional_loop() {
    for limit in [0, 1, 4] {
        assert_eq!(nested_loops(limit), limit);
    }
}
