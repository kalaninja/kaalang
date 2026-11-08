use kaalang::kaa;

/// Shared setup runs before the question; both branches borrow its output.
/// The question itself captures only `condition`.
#[kaa]
fn shared_setup(condition: bool) -> u32 {
    #[kaa::action("Prepare the shared setup.")]
    let setup = || 10;

    #[kaa::question("Take the short branch?")]
    let (short, long) = |condition| condition;

    #[kaa::action("Use the setup on the short branch.")]
    let end = |short, &setup| setup + 1;

    #[kaa::action("Use the setup on the long branch.")]
    let end = |long, &setup| setup + 2;

    |end| return end;
}

#[test]
fn both_selected_actions_borrow_the_shared_setup() {
    assert_eq!(shared_setup(true), 11);
    assert_eq!(shared_setup(false), 12);
}
