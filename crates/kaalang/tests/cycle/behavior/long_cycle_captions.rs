use kaalang::kaa;

#[kaa]
fn long_cycle_captions(flag: bool) {
    #[kaa::question("Choose a cycle.")]
    let (left, right) = |flag| flag;

    #[kaa::cycle(
        "Collect the available results from the source until there are enough items to complete the current request."
    )]
    let done = |left| loop {
        #[kaa::action("Collect the results.")]
        let done = |left| {};
    };

    #[kaa::cycle("Read the next item and prepare it for processing.")]
    let done = |right| loop {
        #[kaa::action("Read the item.")]
        let done = |right| {};
    };

    |done| return;
}

#[test]
fn both_sibling_cycles_complete() {
    long_cycle_captions(true);
    long_cycle_captions(false);
}
