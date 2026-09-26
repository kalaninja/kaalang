use kaalang::kaalang;

#[kaalang]
const fn empty_captures() -> usize {
    #[cycle("Complete without a header.")]
    let first = {
        #[action("Produce the first unit.")]
        let first = || {};
    };

    #[cycle("Complete through an explicit empty capture list.")]
    let second = || {
        #[action("Produce the second unit.")]
        let second = || {};
    };

    #[action("Continue after both cycles.")]
    let result = |first, second| 7;

    |result| return result;
}

#[test]
fn unconditional_cycles_complete_with_their_unit_outputs() {
    const SEVEN: usize = empty_captures();
    assert_eq!(SEVEN, 7);
}
