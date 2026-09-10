use kaalang::kaalang;

/// An owner and the reference borrowed from it stay usable inside their branch.
#[kaalang]
fn borrow_within_branch(condition: bool) -> usize {
    #[question("Build the text?")]
    let (yes, no) = |condition| condition;

    #[action("Build text.")]
    let text = |yes| String::from("hello");

    #[action("Borrow text.")]
    let view = |&text| text.as_str();

    #[action("Log the view.")]
    |view| {
        assert_eq!(view, "hello");
    };

    #[action("Finish yes.")]
    let end = |text| text.len();

    #[action("Finish no.")]
    let end = |no| 0;
}

#[test]
fn the_owner_outlives_the_view_it_lends() {
    assert_eq!(borrow_within_branch(true), 5);
    assert_eq!(borrow_within_branch(false), 0);
}
