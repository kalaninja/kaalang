use kaalang::kaa;

#[kaa]
fn nested_unconditional_cycles(limit: usize) -> usize {
    #[kaa::cycle("Run the nested counter.")]
    let result = |limit| loop {
        #[kaa::action("Initialize the counter.")]
        let mut count = || 0;

        #[kaa::cycle("Count to the limit.")]
        let done = loop {
            #[kaa::question("Has the counter reached the limit?")]
            let (done, again) = |&count, &limit| *count == *limit;

            #[kaa::action("Increment the counter.")]
            |again, &mut count| *count += 1;

            |again| continue;
        };

        #[kaa::action("Hand over the count.")]
        let result = |done, count| count;
    };

    |result| return result;
}

#[test]
fn an_inner_cycle_uses_a_wire_local_to_its_enclosing_iteration() {
    for limit in [0, 1, 10] {
        assert_eq!(nested_unconditional_cycles(limit), limit);
    }
}
