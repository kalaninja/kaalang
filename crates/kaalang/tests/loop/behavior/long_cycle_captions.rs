use kaalang::kaalang;

#[kaalang]
fn long_cycle_captions(flag: bool) {
    #[question("Choose a cycle.")]
    let (left, right) = |flag| flag;

    #[cycle(
        "Collect the available results from the source until there are enough items to complete the current request."
    )]
    let done = |left| {
        #[action("Collect the results.")]
        let ready = |left| {};

        |ready| break;
    };

    #[cycle("Read the next item and prepare it for processing.")]
    let done = |right| {
        #[action("Read the item.")]
        let ready = |right| {};

        |ready| break;
    };

    |done| return;
}

#[test]
fn both_sibling_cycles_complete() {
    long_cycle_captions(true);
    long_cycle_captions(false);
}
