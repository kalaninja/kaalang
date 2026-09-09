use kaalang::kaalang;

/// Borrowing a temporary in an output initializer keeps it alive while the
/// next branch-local action measures it.
#[kaalang]
fn temporary_borrow_within_branch(condition: bool) -> usize {
    #[question("Build the temporary text?")]
    |condition| -> (yes, no) { condition };

    #[action("Borrow the temporary text.")]
    |yes| -> owner { &String::from("abc") };

    #[action("Measure the borrowed text.")]
    |&owner| -> size { owner.len() };

    #[action("Use an empty size.")]
    |no| -> size { 0usize };

    #[action("Finish with the merged size.")]
    |size| -> result { size };
}

#[test]
fn the_temporary_outlives_its_branch_local_use() {
    assert_eq!(temporary_borrow_within_branch(true), 3);
    assert_eq!(temporary_borrow_within_branch(false), 0);
}
