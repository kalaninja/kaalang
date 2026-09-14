use kaalang::kaalang;

#[kaalang]
const fn const_alternative_producers(condition: bool) -> u8 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes value and an unused marker.")]
    let (end, _marker) = |yes| (1, 3u8);

    #[action("Build the no value and an unused marker.")]
    let (end, _marker) = |no| (2, 4);

    |end| return end;
}

#[test]
fn alternative_producer_type_checks_allow_constant_evaluation() {
    const YES: u8 = const_alternative_producers(true);
    const NO: u8 = const_alternative_producers(false);
    assert_eq!((YES, NO), (1, 2));
}
