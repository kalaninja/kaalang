use kaalang::kaalang;

/// An owner and the reference borrowed from it stay usable inside their branch.
#[kaalang]
fn borrow_within_branch(condition: bool) -> usize {
    #[question("Build the text?")]
    |condition| -> (yes, no) { condition };

    #[action("Build text.")]
    |yes| -> text { String::from("hello") };

    #[action("Borrow text.")]
    |&text| -> view { text.as_str() };

    #[action("Log the view.")]
    |view| {
        assert_eq!(view, "hello");
    };

    #[action("Finish yes.")]
    |text| -> result { text.len() };

    #[action("Finish no.")]
    |no| -> result { 0 };
}

#[test]
fn the_owner_outlives_the_view_it_lends() {
    assert_eq!(borrow_within_branch(true), 5);
    assert_eq!(borrow_within_branch(false), 0);
}
