use kaalang::kaa;

#[kaa]
const fn empty_captures() -> usize {
    #[kaa::cycle("Complete without a header.")]
    let first = loop {
        #[kaa::action("Produce the first unit.")]
        let first = || {};
    };

    #[kaa::cycle("Complete through an explicit empty capture list.")]
    let second = || loop {
        #[kaa::action("Produce the second unit.")]
        let second = || {};
    };

    #[kaa::action("Continue after both cycles.")]
    let result = |first, second| 7;

    |result| return result;
}

#[test]
fn unconditional_cycles_complete_with_their_unit_outputs() {
    const SEVEN: usize = empty_captures();
    assert_eq!(SEVEN, 7);
}
