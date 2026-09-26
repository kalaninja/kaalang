use kaalang::kaalang;

#[kaalang]
fn nested_unconditional_loops(limit: usize) -> usize {
    #[cycle("Run the nested counter.")]
    let result = |limit| {
        #[action("Initialize the counter.")]
        let mut count = || 0;

        #[cycle("Count to the limit.")]
        let done = {
            #[question("Has the counter reached the limit?")]
            let (done, again) = |&count, &limit| *count == *limit;

            #[action("Increment the counter.")]
            |again, &mut count| *count += 1;

            |again| continue;
        };

        #[action("Hand over the count.")]
        let result = |done, count| count;
    };

    |result| return result;
}

#[test]
fn an_inner_loop_uses_a_wire_local_to_its_enclosing_iteration() {
    for limit in [0, 1, 10] {
        assert_eq!(nested_unconditional_loops(limit), limit);
    }
}
