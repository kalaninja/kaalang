use kaalang::kaalang;

#[kaalang]
const fn const_alternative_producers(condition: bool) -> u8 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value and an unused marker.")]
    |yes| -> (result, _marker) { (1u8, 3u8) };

    #[action("Build the no value and an unused marker.")]
    |no| -> (result, _marker) { (2u8, 4u8) };
}

#[test]
fn alternative_producer_type_checks_allow_constant_evaluation() {
    const YES: u8 = const_alternative_producers(true);
    const NO: u8 = const_alternative_producers(false);
    assert_eq!((YES, NO), (1, 2));
}
