use kaalang::kaa;

#[kaa]
fn staged_convergence(condition: bool) -> (u32, &'static str) {
    #[kaa::question("Choose two values.")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Build the yes values.")]
    let (number, label) = |yes| (1, "yes");

    #[kaa::action("Build the no values.")]
    let (number, label) = |no| (2, "no");

    #[kaa::action("Use the number first.")]
    let doubled = |number| number * 2;

    |doubled, label| return (doubled, label);
}

#[test]
fn convergence_preserves_wires_for_later_shared_consumers() {
    assert_eq!(staged_convergence(true), (2, "yes"));
    assert_eq!(staged_convergence(false), (4, "no"));
}
