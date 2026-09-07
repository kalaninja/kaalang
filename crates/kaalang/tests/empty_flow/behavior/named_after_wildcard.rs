use kaalang::kaalang;

#[kaalang]
fn named_after_wildcard<T>(_: (), result: T) -> T {}

#[test]
fn end_captures_the_result_named_after_a_wildcard() {
    assert_eq!(named_after_wildcard((), 3), 3);
}
