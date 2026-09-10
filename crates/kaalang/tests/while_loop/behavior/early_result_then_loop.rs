use kaalang::kaalang;

#[kaalang]
fn early_result_then_loop(first: bool, mut count: usize) -> usize {
    #[question("Check for an early result?")]
    while (|first| first) {
        #[action("Return the early result.")]
        let result = || 99;
    }

    #[question("Is the count below three?")]
    while (|count| count < 3) {
        #[action("Increment the count.")]
        |&mut count| *count += 1;
    }

    #[action("Return the count.")]
    let result = |count| count;
}

#[test]
fn a_later_loop_runs_only_when_the_first_loop_finishes_normally() {
    assert_eq!(early_result_then_loop(true, 0), 99);
    assert_eq!(early_result_then_loop(false, 0), 3);
    assert_eq!(early_result_then_loop(true, 1), 99);
    assert_eq!(early_result_then_loop(false, 1), 3);
    assert_eq!(early_result_then_loop(true, 5), 99);
    assert_eq!(early_result_then_loop(false, 5), 5);
}
