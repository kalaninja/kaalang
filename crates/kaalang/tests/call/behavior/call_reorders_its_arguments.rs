use kaalang::kaalang;

mod math {
    pub(super) fn difference(left: i32, right: i32) -> i32 {
        left - right
    }
}

#[kaalang]
fn call_reorders_its_arguments(left: i32, right: i32) -> i32 {
    #[call("Subtract the left value from the right one.")]
    let end = |left, right| math::difference(right, left);

    |end| return end;
}

#[test]
fn the_argument_list_decides_where_each_wire_lands() {
    assert_eq!(call_reorders_its_arguments(10, 4), -6);
    assert_eq!(call_reorders_its_arguments(4, 10), 6);
}
