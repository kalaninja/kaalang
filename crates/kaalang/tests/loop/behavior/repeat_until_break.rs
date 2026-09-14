use kaalang::kaalang;

#[kaalang]
fn repeat_until_break(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut count = || 0;

    #[cycle("Count until the limit is reached.")]
    let result = |mut count, limit| {
        #[question("Has the counter reached the limit?")]
        let (done, again) = |&count, &limit| *count == *limit;

        |done, count| break count;

        #[action("Increment the counter.")]
        |again, &mut count| *count += 1;
    };

    |result| return result;
}

#[test]
fn repeats_until_a_branch_completes_the_cycle() {
    for limit in [0, 1, 10] {
        assert_eq!(repeat_until_break(limit), limit);
    }
}
