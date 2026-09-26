use kaalang::kaalang;

#[kaalang]
fn count_to(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut counter = || 0usize;

    #[action("Begin counting.")]
    let count = || {};

    #[stage("Count to the limit.")]
    let (count, finish) = |count| {
        #[question("Done?")]
        let (finish, again) = |counter, limit| counter >= limit;

        #[action("Increment and repeat.")]
        let count = |again, &mut counter| {
            *counter += 1;
        };
    };

    #[stage("Return the count.")]
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
