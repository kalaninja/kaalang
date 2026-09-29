use kaalang::kaalang;
#[kaalang]
fn clone_initial_signal_by_reference(go: String) -> String {
    #[action("Clone through a shared borrow.")]
    let copy = |&go| go.clone();
    #[action("Use the clone.")]
    |copy| drop(copy);
    #[stage("Return the original.")]
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
