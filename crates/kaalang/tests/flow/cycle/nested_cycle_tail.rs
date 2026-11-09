use kaalang::kaa;

#[kaa]
fn nested_cycle_tail(mut count: usize, limit: usize) -> usize {
    #[kaa::cycle("Count to the limit through an inner cycle.")]
    let leave_1 = loop {
        #[kaa::question("Is another counting pass needed?")]
        #[kaa::no("NO")]
        #[kaa::yes("YES")]
        let (leave_1, iterate_1) = |&count, &limit| *count < *limit;

        #[kaa::cycle("Increment until the current pass is complete.")]
        let leave_2 = |iterate_1| loop {
            #[kaa::question("Is the count below the limit?")]
            #[kaa::yes("YES")]
            #[kaa::no("NO")]
            let (iterate_2, leave_2) = |&count, &limit| *count < *limit;

            #[kaa::action("Increment the count.")]
            |iterate_2, &mut count| *count += 1;

            |iterate_2| continue;
        };

        |leave_2| continue;
    };

    |leave_1, count| return count;
}

#[test]
fn inner_exit_can_finish_the_outer_iteration() {
    assert_eq!(nested_cycle_tail(0, 0), 0);
    assert_eq!(nested_cycle_tail(0, 4), 4);
    assert_eq!(nested_cycle_tail(5, 4), 5);
}
