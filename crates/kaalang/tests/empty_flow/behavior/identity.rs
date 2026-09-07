use kaalang::kaalang;

#[kaalang]
fn identity<T>(result: T) -> T {}

#[test]
fn end_captures_the_flow_input_named_result() {
    assert_eq!(identity(String::from("value")), "value");
}
