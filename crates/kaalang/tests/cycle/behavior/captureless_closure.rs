use kaalang::kaalang;

#[kaalang]
fn captureless_closure() -> u8 {
    #[action("Build a closure without captures.")]
    let f = || |x: u8| x + 1;

    #[action("Call the closure.")]
    let result = |f| f(6);

    |result| return result;
}

#[test]
fn an_initializer_without_captures_can_contain_a_closure() {
    assert_eq!(captureless_closure(), 7);
}
