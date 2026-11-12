use kaalang::kaa;

#[kaa]
const fn nearest_cycle_outputs(mut count: usize) -> usize {
    #[kaa::cycle("Run passes until two have finished.")]
    let done = loop {
        #[kaa::cycle("Complete the inner pass immediately.")]
        let finished = || loop {
            #[kaa::action("Finish the inner pass.")]
            let finished = || {};
        };

        #[kaa::action("Advance the outer pass.")]
        let advanced = |finished, &mut count| *count += 1;

        #[kaa::question("Have two passes finished?")]
        let (done, again) = |advanced, &count| *count >= 2;

        #[kaa::action("Finish this pass.")]
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
