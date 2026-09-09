use kaalang::kaalang;

/// A borrow above a move. Swapping the two blocks would move the string before
/// the borrow, which Rust rejects; kaalang itself imposes no order between them.
#[kaalang]
fn log_then_transform(value: String) -> usize {
    #[action("Log the value.")]
    |&value| {
        assert_eq!(value, "abc");
    };

    #[action("Transform the value into its length.")]
    |value| -> length { value.len() };

    #[action("Finish.")]
    |length| -> result { length };
}

#[test]
fn the_borrow_reads_the_value_before_the_move() {
    assert_eq!(log_then_transform(String::from("abc")), 3);
}
