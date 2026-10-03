use kaalang::kaalang;

#[kaalang]
fn long_cycle_captions(flag: bool) {
    #[question("Choose a cycle.")]
    let (left, right) = |flag| flag;

    #[cycle(
        "Collect the available results from the source until there are enough items to complete the current request."
    )]
    let done = |left| loop {
        #[action("Collect the results.")]
        let done = |left| {};
    };

    #[cycle("Read the next item and prepare it for processing.")]
    let done = |right| loop {
        #[action("Read the item.")]
        let done = |right| {};
    };

    |done| return;
}

#[test]
fn both_sibling_cycles_complete() {
    long_cycle_captions(true);
    long_cycle_captions(false);
}
