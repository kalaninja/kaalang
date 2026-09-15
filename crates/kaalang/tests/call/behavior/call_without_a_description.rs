use kaalang::kaalang;

fn double(value: i32) -> i32 {
    value * 2
}

#[kaalang]
fn call_without_a_description(value: i32) -> i32 {
    #[call]
    let end = |value| double(value);

    |end| return end;
}

#[test]
fn a_call_without_a_description_is_named_by_its_function() {
    assert_eq!(call_without_a_description(21), 42);
}
