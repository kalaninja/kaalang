use std::cell::RefCell;

use kaalang::kaa;

/// Repeating `_guard` declares a merge even though no later block captures it.
/// The selected guard stays alive through common work and drops at flow exit.
#[kaa]
fn unused_guard_survives_a_merge(condition: bool, cell: &RefCell<usize>) -> bool {
    #[kaa::question("Which branch borrows the cell?")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Borrow the cell on the yes branch.")]
    let _guard = |yes, cell| cell.borrow_mut();

    #[kaa::action("Borrow the cell on the no branch.")]
    let _guard = |no, cell| cell.borrow_mut();

    #[kaa::action("Check that the merged guard is still alive.")]
    let end = |cell| cell.try_borrow_mut().is_err();

    |end| return end;
}

#[test]
fn an_uncaptured_merged_guard_lives_until_flow_exit() {
    for condition in [true, false] {
        let cell = RefCell::new(3);
        assert!(unused_guard_survives_a_merge(condition, &cell));
        assert!(cell.try_borrow_mut().is_ok());
    }
}
