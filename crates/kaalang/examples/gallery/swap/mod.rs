use kaalang::kaa;

#[kaa]
fn swap<T>(a: T, b: T) -> (T, T) {
    |a, b| return (b, a);
}

pub(super) fn check() {
    assert_eq!(swap(1, 2), (2, 1));
    assert_eq!(swap("left", "right"), ("right", "left"));
}
