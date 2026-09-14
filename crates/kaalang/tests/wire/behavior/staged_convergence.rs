use kaalang::kaalang;

#[kaalang]
fn staged_convergence(condition: bool) -> (u32, &'static str) {
    #[question("Choose two values.")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes values.")]
    let (number, label) = |yes| (1, "yes");

    #[action("Build the no values.")]
    let (number, label) = |no| (2, "no");

    #[action("Use the number first.")]
    let doubled = |number| number * 2;

    |doubled, label| return (doubled, label);
}

#[test]
fn convergence_preserves_wires_for_later_shared_consumers() {
    assert_eq!(staged_convergence(true), (2, "yes"));
    assert_eq!(staged_convergence(false), (4, "no"));
}
