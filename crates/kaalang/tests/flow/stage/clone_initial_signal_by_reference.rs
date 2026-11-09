use kaalang::kaa;
#[kaa]
fn clone_initial_signal_by_reference(go: String) -> String {
    #[kaa::action("Clone through a shared borrow.")]
    let copy = |&go| go.clone();
    #[kaa::action("Use the clone.")]
    |copy| drop(copy);
    #[kaa::stage("Return the original.")]
    |go| {
        |go| return go;
    };
}
#[test]
fn a_borrowed_clone_preserves_the_initial_signal() {
    assert_eq!(
        clone_initial_signal_by_reference(String::from("first")),
        "first"
    );
}
