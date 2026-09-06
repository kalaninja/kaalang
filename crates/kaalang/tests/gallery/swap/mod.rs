use kaalang::kaalang;

#[kaalang]
fn swap<T>(a: T, b: T) -> (T, T) {
    #[action("Exchange the pair.")]
    |a, b| -> result { (b, a) };
}

#[test]
fn swap_exchanges_the_pair() {
    assert_eq!(swap(1, 2), (2, 1));
}
