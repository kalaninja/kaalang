use kaalang::kaalang;

#[kaalang]
fn capturing_closure_after_convergence(condition: bool) -> u32 {
    #[question("Which base?")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes base.")]
    let base = |yes| 10;

    #[action("Build the no base.")]
    let base = |no| 20;

    #[action("Capture the selected base in a closure.")]
    let add = |base| move |delta: u32| base + delta;

    #[action("Apply the closure.")]
    let end = |add| add(1);
}

#[test]
fn a_shared_closure_captures_the_branch_bound_value() {
    assert_eq!(capturing_closure_after_convergence(true), 11);
    assert_eq!(capturing_closure_after_convergence(false), 21);
}
