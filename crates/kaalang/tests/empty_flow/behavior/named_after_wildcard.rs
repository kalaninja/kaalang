use kaalang::kaalang;

#[kaalang]
fn named_after_wildcard<T>(_: (), end: T) -> T {
    |end| return end;
}

#[test]
fn return_captures_the_wire_named_after_a_wildcard() {
    assert_eq!(named_after_wildcard((), 3), 3);
}
