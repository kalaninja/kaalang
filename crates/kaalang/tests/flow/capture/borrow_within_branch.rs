use kaalang::kaa;

/// An owner and the reference borrowed from it stay usable inside their branch.
#[kaa]
fn borrow_within_branch(condition: bool) -> usize {
    #[kaa::question("Build the text?")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Build text.")]
    let text = |yes| String::from("hello");

    #[kaa::action("Borrow text.")]
    let view = |&text| text.as_str();

    #[kaa::action("Log the view.")]
    |view| {
        assert_eq!(view, "hello");
    };

    #[kaa::action("Finish yes.")]
    let end = |text| text.len();

    #[kaa::action("Finish no.")]
    let end = |no| 0;

    |end| return end;
}

#[test]
fn the_owner_outlives_the_view_it_lends() {
    assert_eq!(borrow_within_branch(true), 5);
    assert_eq!(borrow_within_branch(false), 0);
}
