use kaalang::kaalang;

/// Source order places the borrow before the move. Swapping them makes Rust
/// reject the borrow of the moved string.
#[kaalang]
fn log_then_transform(value: String) -> usize {
    #[action("Log the value.")]
    |&value| {
        assert_eq!(value, "abc");
    };

    #[action("Transform the value into its length.")]
    let length = |value| value.len();

    |length| return length;
}

#[test]
fn the_borrow_reads_the_value_before_the_move() {
    assert_eq!(log_then_transform(String::from("abc")), 3);
}
