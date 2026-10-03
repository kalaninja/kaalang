use kaalang::kaalang;

#[kaalang]
const fn nearest_cycle_outputs(mut count: usize) -> usize {
    #[cycle("Run passes until two have finished.")]
    let done = loop {
        #[cycle("Complete the inner pass immediately.")]
        let finished = || loop {
            #[action("Finish the inner pass.")]
            let finished = || {};
        };

        #[action("Advance the outer pass.")]
        let advanced = |finished, &mut count| *count += 1;

        #[question("Have two passes finished?")]
        let (done, again) = |advanced, &count| *count >= 2;

        #[action("Finish this pass.")]
        |again| {};

        |again| continue;
    };

    |done, count| return count;
}

#[test]
fn each_output_completes_its_directly_enclosing_cycle() {
    const TWO: usize = nearest_cycle_outputs(0);
    assert_eq!(TWO, 2);
}
