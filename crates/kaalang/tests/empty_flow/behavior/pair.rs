use kaalang::kaalang;

#[kaalang]
fn pair<T>(a: T, b: T) -> (T, T) {
    #[end]
    |a, b| {};
}

#[test]
fn end_captures_sources_in_authored_order() {
    assert_eq!(pair(1, 2), (1, 2));
}
