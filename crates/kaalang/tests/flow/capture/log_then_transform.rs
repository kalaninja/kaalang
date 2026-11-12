use kaalang::kaa;

/// Source order places the borrow before the move. Swapping them makes Rust
/// reject the borrow of the moved string.
#[kaa]
fn log_then_transform(value: String) -> usize {
    #[kaa::action("Log the value.")]
    |&value| {
        assert_eq!(value, "abc");
    };

    #[kaa::action("Transform the value into its length.")]
    let length = |value| value.len();

    |length| return length;
}

#[test]
fn the_borrow_reads_the_value_before_the_move() {
    assert_eq!(log_then_transform(String::from("abc")), 3);
}
