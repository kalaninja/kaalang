use kaalang::kaalang;

#[kaalang]
fn several_wires(condition: bool) -> (u32, &'static str) {
    #[question("Choose a pair.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes pair.")]
    |yes| -> (number, label) { (1, "yes") };

    #[action("Build the no pair.")]
    |no| -> (number, label) { (2, "no") };

    #[action("Use both selected values.")]
    |label, number| -> result { (number, label) };

    #[end]
    |result| {};
}

#[test]
fn convergence_carries_several_wires_in_consumer_order() {
    assert_eq!(several_wires(true), (1, "yes"));
    assert_eq!(several_wires(false), (2, "no"));
}
