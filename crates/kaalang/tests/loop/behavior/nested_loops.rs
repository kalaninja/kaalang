use kaalang::kaalang;

#[kaalang]
fn nested_loops(limit: usize) -> usize {
    #[action("Initialize the outer counter.")]
    let mut outer = || 0;

    loop {
        #[action("Initialize the iteration counter.")]
        let mut inner = || 0;

        #[question("Is the iteration counter below the outer counter?")]
        while (|&inner, &outer| *inner < *outer) {
            #[action("Increment the iteration counter.")]
            |&mut inner| *inner += 1;
        }

        #[question("Has the outer counter reached the limit?")]
        let (done, again) = |&outer, &limit| *outer == *limit;

        #[action("Return the counter.")]
        let end = |done, &outer| *outer;

        #[action("Increment the outer counter.")]
        |again, &mut outer| *outer += 1;
    }
}

#[test]
fn nests_a_while_inside_an_unconditional_loop() {
    for limit in [0, 1, 4] {
        assert_eq!(nested_loops(limit), limit);
    }
}
