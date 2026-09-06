use kaalang::kaalang;

#[kaalang]
fn staged_convergence(condition: bool) -> (u32, &'static str) {
    #[question("Choose two values.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes values.")]
    |yes| -> (number, label) { (1, "yes") };

    #[action("Build the no values.")]
    |no| -> (number, label) { (2, "no") };

    #[action("Use the number first.")]
    |number| -> doubled { number * 2 };

    #[action("Use the preserved label later.")]
    |doubled, label| -> result { (doubled, label) };
}

#[test]
fn convergence_preserves_wires_for_later_shared_consumers() {
    assert_eq!(staged_convergence(true), (2, "yes"));
    assert_eq!(staged_convergence(false), (4, "no"));
}
