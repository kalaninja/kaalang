use kaalang::kaalang;

#[kaalang]
const fn nearest_cycle_breaks(count: usize) -> usize {
    #[cycle("Run passes until two have finished.")]
    let result = |mut count| {
        #[cycle("Complete the inner pass immediately.")]
        || {
            break;
        };

        #[action("Advance the outer pass.")]
        let advanced = |&mut count| *count += 1;

        #[question("Have two passes finished?")]
        let (done, again) = |advanced, &count| *count >= 2;

        |done, count| break count;

        #[action("Finish this pass.")]
        |again| {};
    };

    |result| return result;
}

#[test]
fn each_break_finishes_its_directly_enclosing_cycle() {
    const TWO: usize = nearest_cycle_breaks(0);
    assert_eq!(TWO, 2);
}
