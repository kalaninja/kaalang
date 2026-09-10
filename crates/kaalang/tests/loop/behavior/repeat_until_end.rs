use kaalang::kaalang;

#[kaalang]
fn repeat_until_end(limit: usize) -> usize {
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

#[test]
fn repeats_until_a_branch_finishes_the_flow() {
    for limit in [0, 1, 10] {
        assert_eq!(repeat_until_end(limit), limit);
    }
}
