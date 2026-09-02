use kaalang::kaalang;

#[kaalang]
fn reverse_pair<T>(a: T, b: T) -> (T, T) {
    #[end]
    |b, a| {};
}

#[test]
fn end_captures_sources_in_authored_order() {
    assert_eq!(reverse_pair(1, 2), (2, 1));
}
