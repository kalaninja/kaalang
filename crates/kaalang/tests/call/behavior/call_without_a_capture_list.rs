use kaalang::kaalang;

fn origin() -> (i32, i32) {
    (0, 0)
}

#[kaalang]
fn call_without_a_capture_list() -> (i32, i32) {
    #[call("Start at the origin.")]
    let end = origin();

    |end| return end;
}

#[test]
fn a_call_that_captures_nothing_omits_the_empty_capture_list() {
    assert_eq!(call_without_a_capture_list(), (0, 0));
}
