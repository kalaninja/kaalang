use kaalang::kaa;

/// Two borrows and then a move of the same wire, in exactly that order. The
/// move also waits for a trigger the first borrow produced. Rust checks the
/// borrows against the move; kaalang only fixes the sequence.
#[kaa]
fn borrowers_before_a_move(text: String) -> (usize, bool, String) {
    #[kaa::action("Measure the text.")]
    let (length, trigger) = |&text| (text.len(), ());

    #[kaa::action("Ask whether the text is empty.")]
    let empty = |&text| text.is_empty();

    |trigger, text, length, empty| return (length, empty, text);
}

#[test]
fn the_borrows_finish_before_the_triggered_move() {
    assert_eq!(
        borrowers_before_a_move(String::from("abc")),
        (3, false, String::from("abc"))
    );
}
