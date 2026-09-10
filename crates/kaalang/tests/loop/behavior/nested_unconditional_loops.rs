use kaalang::kaalang;

#[kaalang]
fn nested_unconditional_loops(limit: usize) -> usize {
    loop {
        #[action("Initialize the counter.")]
        let mut count = || 0;

        loop {
            #[question("Has the counter reached the limit?")]
            let (done, again) = |&count, &limit| *count == *limit;

            #[action("Return the counter.")]
            let end = |done, &count| *count;

            #[action("Increment the counter.")]
            |again, &mut count| *count += 1;
        }
    }
}

#[test]
fn an_inner_loop_uses_a_wire_local_to_its_enclosing_iteration() {
    for limit in [0, 1, 10] {
        assert_eq!(nested_unconditional_loops(limit), limit);
    }
}
