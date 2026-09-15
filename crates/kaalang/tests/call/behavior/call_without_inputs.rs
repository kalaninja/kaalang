use kaalang::kaalang;

fn origin() -> (i32, i32) {
    (0, 0)
}

#[kaalang]
fn call_without_inputs() -> (i32, i32) {
    #[call("Start at the origin.")]
    let end = || origin();

    |end| return end;
}

#[test]
fn a_call_may_take_no_inputs() {
    assert_eq!(call_without_inputs(), (0, 0));
}
