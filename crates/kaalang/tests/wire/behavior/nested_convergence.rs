use kaalang::kaalang;

#[kaalang]
fn nested_convergence(outer: bool, inner: bool) -> u32 {
    #[question("Take the nested branch?")]
    let (nested, direct) = |outer, &inner| outer;

    #[question("Choose the nested value.")]
    let (inner_yes, inner_no) = |nested, inner| inner;

    #[action("Build the nested yes value.")]
    let selected = |inner_yes| 1;

    #[action("Build the nested no value.")]
    let selected = |inner_no| 2;

    #[action("Build the direct value.")]
    let selected = |direct| 3;

    #[action("Use the selected nested value.")]
    let result = |selected| selected * 10;
}

#[test]
fn nested_branches_converge_once() {
    assert_eq!(nested_convergence(true, true), 10);
    assert_eq!(nested_convergence(true, false), 20);
    assert_eq!(nested_convergence(false, true), 30);
}
