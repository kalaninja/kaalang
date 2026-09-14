use kaalang::kaalang;

#[kaalang]
fn identity<T>(end: T) -> T {
    |end| return end;
}

#[test]
fn return_captures_the_flow_input_named_end() {
    assert_eq!(identity(String::from("value")), "value");
}
