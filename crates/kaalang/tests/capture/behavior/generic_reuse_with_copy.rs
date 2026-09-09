use kaalang::kaalang;

/// A `Copy` bound lets Rust accept repeated value captures of a generic wire.
#[kaalang]
fn generic_reuse_with_copy<T: Copy>(value: T) -> (T, T) {
    #[action("Take the value.")]
    |value| -> first { value };

    #[action("Take the value again.")]
    |value| -> second { value };

    #[action("Pair the two values.")]
    |first, second| -> result { (first, second) };
}

#[test]
fn a_copy_bound_allows_repeated_value_captures() {
    assert_eq!(generic_reuse_with_copy(7u32), (7, 7));

    let text = String::from("copy");
    assert_eq!(generic_reuse_with_copy(text.as_str()), ("copy", "copy"));
}
