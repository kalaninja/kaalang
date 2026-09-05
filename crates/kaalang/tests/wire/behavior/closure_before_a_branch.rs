use kaalang::kaalang;

#[kaalang]
fn closure_before_a_branch(condition: bool, base: u32) -> u32 {
    #[question("Add or subtract?")]
    |condition| -> (add, subtract) { condition };

    #[action("Build a handler either branch may consume.")]
    |base| -> handler { move |delta: u32| base + delta };

    #[action("Add one.")]
    |add, handler| -> result { handler(1) };

    #[action("Subtract one.")]
    |subtract, handler| -> result { handler(0) - 1 };

    #[end]
    |result| {};
}

#[test]
fn an_independent_closure_is_bound_once_before_the_branch() {
    assert_eq!(closure_before_a_branch(true, 10), 11);
    assert_eq!(closure_before_a_branch(false, 10), 9);
}
