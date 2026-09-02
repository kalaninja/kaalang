use kaalang::kaalang;

#[kaalang]
fn identity<T>(value: T) -> T {
    #[end]
    |value| {};
}

#[test]
fn end_captures_the_source_by_name() {
    assert_eq!(identity(String::from("value")), "value");
}
