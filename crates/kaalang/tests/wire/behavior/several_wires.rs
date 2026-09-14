use kaalang::kaalang;

#[kaalang]
fn several_wires(condition: bool) -> (u32, &'static str) {
    #[question("Choose a pair.")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes pair.")]
    let (number, label) = |yes| (1, "yes");

    #[action("Build the no pair.")]
    let (number, label) = |no| (2, "no");

    |label, number| return (number, label);
}

#[test]
fn convergence_carries_several_wires_in_consumer_order() {
    assert_eq!(several_wires(true), (1, "yes"));
    assert_eq!(several_wires(false), (2, "no"));
}
