use kaalang::kaa;

#[kaa]
fn nested_convergence(outer: bool, inner: bool) -> u32 {
    #[kaa::question("Take the nested branch?")]
    let (nested, direct) = |outer, &inner| outer;

    #[kaa::question("Choose the nested value.")]
    let (inner_yes, inner_no) = |nested, inner| inner;

    #[kaa::action("Build the nested yes value.")]
    let selected = |inner_yes| 1;

    #[kaa::action("Build the nested no value.")]
    let selected = |inner_no| 2;

    #[kaa::action("Build the direct value.")]
    let selected = |direct| 3;

    #[kaa::action("Use the selected nested value.")]
    let end = |selected| selected * 10;

    |end| return end;
}

#[test]
fn nested_branches_converge_once() {
    assert_eq!(nested_convergence(true, true), 10);
    assert_eq!(nested_convergence(true, false), 20);
    assert_eq!(nested_convergence(false, true), 30);
}
