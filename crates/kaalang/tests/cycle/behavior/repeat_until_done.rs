use kaalang::kaalang;

#[kaalang]
fn repeat_until_done(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut count = || 0;

    #[cycle("Count until the limit is reached.")]
    let done = loop {
        #[question("Has the counter reached the limit?")]
        let (done, again) = |&count, &limit| *count == *limit;

        #[action("Increment the counter.")]
        |again, &mut count| *count += 1;

        |again| continue;
    };

    |done, count| return count;
}

#[test]
fn repeats_until_a_branch_completes_the_cycle() {
    for limit in [0, 1, 10] {
        assert_eq!(repeat_until_done(limit), limit);
    }
}
