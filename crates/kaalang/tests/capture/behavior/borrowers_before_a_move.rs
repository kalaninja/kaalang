use kaalang::kaalang;

/// Two borrows and then a move of the same wire, in exactly that order. The
/// move also waits for a trigger the first borrow produced. Rust checks the
/// borrows against the move; kaalang only fixes the sequence.
#[kaalang]
fn borrowers_before_a_move(text: String) -> (usize, bool, String) {
    #[action("Measure the text.")]
    let (length, trigger) = |&text| (text.len(), ());

    #[action("Ask whether the text is empty.")]
    let empty = |&text| text.is_empty();

    #[action("Take the text once the trigger and both borrows are done.")]
    let result = |trigger, text, length, empty| (length, empty, text);
}

#[test]
fn the_borrows_finish_before_the_triggered_move() {
    assert_eq!(
        borrowers_before_a_move(String::from("abc")),
        (3, false, String::from("abc"))
    );
}
