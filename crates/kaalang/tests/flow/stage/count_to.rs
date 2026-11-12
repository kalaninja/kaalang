use kaalang::kaa;

#[kaa]
fn count_to(limit: usize) -> usize {
    #[kaa::action("Initialize the counter.")]
    let mut counter = || 0usize;

    #[kaa::action("Begin counting.")]
    let count = || {};

    #[kaa::stage("Count to the limit.")]
    let (count, finish) = |count| {
        #[kaa::question("Done?")]
        let (finish, again) = |counter, limit| counter >= limit;

        #[kaa::action("Increment and repeat.")]
        let count = |again, &mut counter| {
            *counter += 1;
        };
    };

    #[kaa::stage("Return the count.")]
    |finish| {
        |counter| return counter;
    };
}

#[test]
fn counts_through_repeated_visits() {
    for limit in [0, 1, 7] {
        assert_eq!(count_to(limit), limit);
    }
}
