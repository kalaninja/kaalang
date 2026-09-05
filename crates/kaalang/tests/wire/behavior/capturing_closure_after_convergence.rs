use kaalang::kaalang;

#[kaalang]
fn capturing_closure_after_convergence(condition: bool) -> u32 {
    #[question("Which base?")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes base.")]
    |yes| -> base { 10 };

    #[action("Build the no base.")]
    |no| -> base { 20 };

    #[action("Capture the selected base in a closure.")]
    |base| -> add { move |delta: u32| base + delta };

    #[action("Apply the closure.")]
    |add| -> result { add(1) };

    #[end]
    |result| {};
}

#[test]
fn a_shared_closure_captures_the_branch_bound_value() {
    assert_eq!(capturing_closure_after_convergence(true), 11);
    assert_eq!(capturing_closure_after_convergence(false), 21);
}
