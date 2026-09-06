use kaalang::kaalang;

#[kaalang]
fn nested_convergence(outer: bool, inner: bool) -> u32 {
    #[question("Take the nested path?")]
    |outer, &inner| -> (nested, direct) { outer };

    #[question("Choose the nested value.")]
    |nested, inner| -> (inner_yes, inner_no) { inner };

    #[action("Build the nested yes value.")]
    |inner_yes| -> selected { 1 };

    #[action("Build the nested no value.")]
    |inner_no| -> selected { 2 };

    #[action("Build the direct value.")]
    |direct| -> selected { 3 };

    #[action("Use the selected nested value.")]
    |selected| -> result { selected * 10 };
}

#[test]
fn nested_paths_converge_once() {
    assert_eq!(nested_convergence(true, true), 10);
    assert_eq!(nested_convergence(true, false), 20);
    assert_eq!(nested_convergence(false, true), 30);
}
