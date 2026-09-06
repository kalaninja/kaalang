use kaalang::kaalang;

#[kaalang]
fn identity<T>(result: T) -> T {}

#[test]
fn end_captures_the_source_by_name() {
    assert_eq!(identity(String::from("value")), "value");
}
