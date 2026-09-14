use kaalang::kaalang;

#[kaalang]
fn closure_before_a_branch(condition: bool, base: u32) -> u32 {
    #[action("Build a handler either branch may consume.")]
    let handler = |base| move |delta: u32| base + delta;

    #[question("Add or subtract?")]
    let (add, subtract) = |condition| condition;

    #[action("Add one.")]
    let end = |add, handler| handler(1);

    #[action("Subtract one.")]
    let end = |subtract, handler| handler(0) - 1;

    |end| return end;
}

#[test]
fn the_shared_closure_is_bound_once_before_the_branch() {
    assert_eq!(closure_before_a_branch(true, 10), 11);
    assert_eq!(closure_before_a_branch(false, 10), 9);
}
