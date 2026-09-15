use kaalang::kaalang;

mod math {
    pub(super) fn difference(left: i32, right: i32) -> i32 {
        left - right
    }
}

#[kaalang]
fn call_a_path(left: i32, right: i32) -> i32 {
    #[call("Subtract the right value from the left one.")]
    let end = |left, right| math::difference(left, right);

    |end| return end;
}

#[test]
fn a_call_names_the_function_it_runs_by_path() {
    assert_eq!(call_a_path(10, 4), 6);
    assert_eq!(call_a_path(4, 10), -6);
}
