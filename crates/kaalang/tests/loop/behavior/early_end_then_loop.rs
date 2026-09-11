use kaalang::kaalang;

#[kaalang]
fn early_end_then_loop(first: bool, mut count: usize) -> usize {
    loop {
        #[question("Check for an early result?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |first| first;

        |leave_1| break;

        #[action("Return the early result.")]
        let end = |iterate_1| 99;
    }

    |&count| loop {
        #[question("Is the count below three?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_2, leave_2) = |count| count < 3;

        |leave_2| break;

        #[action("Increment the count.")]
        |iterate_2, &mut count| *count += 1;
    };

    #[action("Return the count.")]
    let end = |count| count;
}

#[test]
fn a_later_loop_runs_only_when_the_first_loop_finishes_normally() {
    assert_eq!(early_end_then_loop(true, 0), 99);
    assert_eq!(early_end_then_loop(false, 0), 3);
    assert_eq!(early_end_then_loop(true, 1), 99);
    assert_eq!(early_end_then_loop(false, 1), 3);
    assert_eq!(early_end_then_loop(true, 5), 99);
    assert_eq!(early_end_then_loop(false, 5), 5);
}
