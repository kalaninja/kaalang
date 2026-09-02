use kaalang::kaalang;

#[kaalang]
fn named_after_wildcard<T>(_: (), value: T) -> T {
    #[end]
    |value| {};
}

#[test]
fn end_captures_the_source_by_name() {
    assert_eq!(named_after_wildcard((), 3), 3);
}
