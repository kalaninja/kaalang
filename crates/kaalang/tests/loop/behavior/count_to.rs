use kaalang::kaalang;

#[kaalang]
const fn count_to(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut count = || 0;

    |&count, &limit| loop {
        #[question("Is the counter below the limit?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |&count, &limit| *count < *limit;

        |leave_1| break;

        #[action("Increment the counter.")]
        |iterate_1, &mut count| *count += 1;
    };

    #[action("Return the counter.")]
    let end = |count| count;
}

#[test]
fn repeats_until_the_condition_is_false() {
    const THREE: usize = count_to(3);
    assert_eq!(THREE, 3);
    for limit in [0, 1, 10] {
        assert_eq!(count_to(limit), limit);
    }
}
