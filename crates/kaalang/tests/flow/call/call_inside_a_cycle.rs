use kaalang::kaa;

fn bump(count: &mut usize) {
    *count += 1;
}

#[kaa]
fn call_inside_a_cycle(limit: usize) -> usize {
    #[kaa::action("Initialize the counter.")]
    let mut count = || 0;

    #[kaa::cycle("Count to the limit.")]
    let leave_1 = loop {
        #[kaa::question("Is the counter below the limit?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (iterate_1, leave_1) = |&count, &limit| *count < *limit;

        #[kaa::call("Increment the counter.")]
        |iterate_1, &mut count| bump(count);

        |iterate_1| continue;
    };

    |leave_1, count| return count;
}

#[test]
fn a_call_repeats_with_the_cycle_that_encloses_it() {
    for limit in [0, 1, 3, 10] {
        assert_eq!(call_inside_a_cycle(limit), limit);
    }
}
