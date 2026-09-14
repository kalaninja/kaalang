use kaalang::kaalang;

#[kaalang]
fn swap<T>(a: T, b: T) -> (T, T) {
    |a, b| return (b, a);
}

#[test]
fn swap_exchanges_the_pair() {
    assert_eq!(swap(1, 2), (2, 1));
}
