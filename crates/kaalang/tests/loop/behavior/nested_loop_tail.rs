use kaalang::kaalang;

#[kaalang]
fn nested_loop_tail(count: usize, limit: usize) -> usize {
    #[cycle("Count to the limit through an inner cycle.")]
    let result = |mut count, limit| {
        #[question("Is another counting pass needed?")]
        #[no("NO")]
        #[yes("YES")]
        let (leave_1, iterate_1) = |&count, &limit| *count < *limit;

        |leave_1, count| break count;

        #[cycle("Increment until the current pass is complete.")]
        |iterate_1, &mut count, limit| {
            #[question("Is the count below the limit?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_2, leave_2) = |&count, &limit| **count < *limit;

            |leave_2| break;

            #[action("Increment the count.")]
            |iterate_2, &mut count| **count += 1;
        };
    };

    |result| return result;
}

#[test]
fn inner_exit_can_finish_the_outer_iteration() {
    assert_eq!(nested_loop_tail(0, 0), 0);
    assert_eq!(nested_loop_tail(0, 4), 4);
    assert_eq!(nested_loop_tail(5, 4), 5);
}
