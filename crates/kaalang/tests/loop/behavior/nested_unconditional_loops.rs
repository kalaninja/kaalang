use kaalang::kaalang;

#[kaalang]
fn nested_unconditional_loops(limit: usize) -> usize {
    #[cycle("Run the nested counter.")]
    let result = |limit| {
        #[action("Initialize the counter.")]
        let mut count = || 0;

        #[cycle("Count to the limit.")]
        let final_count = |mut count, limit| {
            #[question("Has the counter reached the limit?")]
            let (done, again) = |&count, &limit| *count == *limit;

            |done, count| break count;

            #[action("Increment the counter.")]
            |again, &mut count| *count += 1;
        };

        |final_count| break final_count;
    };

    |result| return result;
}

#[test]
fn an_inner_loop_uses_a_wire_local_to_its_enclosing_iteration() {
    for limit in [0, 1, 10] {
        assert_eq!(nested_unconditional_loops(limit), limit);
    }
}
