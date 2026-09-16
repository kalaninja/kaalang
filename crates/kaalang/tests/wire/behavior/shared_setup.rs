use kaalang::kaalang;

/// Shared setup runs before the question; both branches borrow its output.
/// The question itself captures only `condition`.
#[kaalang]
fn shared_setup(condition: bool) -> u32 {
    #[action("Prepare the shared setup.")]
    let setup = || 10;

    #[question("Take the short branch?")]
    let (short, long) = |condition| condition;

    #[action("Use the setup on the short branch.")]
    let end = |short, &setup| setup + 1;

    #[action("Use the setup on the long branch.")]
    let end = |long, &setup| setup + 2;

    |end| return end;
}

#[test]
fn both_selected_actions_borrow_the_shared_setup() {
    assert_eq!(shared_setup(true), 11);
    assert_eq!(shared_setup(false), 12);
}
