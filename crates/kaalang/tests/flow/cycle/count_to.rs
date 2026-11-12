use kaalang::kaa;

#[kaa]
const fn count_to(limit: usize) -> usize {
    #[kaa::action("Initialize the counter.")]
    let mut count = || 0;

    #[kaa::cycle("Count to the limit.")]
    let leave_1 = loop {
        #[kaa::question("Is the counter below the limit?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (iterate_1, leave_1) = |&count, &limit| *count < *limit;

        #[kaa::action("Increment the counter.")]
        |iterate_1, &mut count| *count += 1;

        |iterate_1| continue;
    };

    |leave_1, count| return count;
}

#[test]
fn repeats_until_the_condition_is_false() {
    const THREE: usize = count_to(3);
    assert_eq!(THREE, 3);
    for limit in [0, 1, 10] {
        assert_eq!(count_to(limit), limit);
    }
}
