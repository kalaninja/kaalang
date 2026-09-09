use std::cell::RefCell;

use kaalang::kaalang;

/// Repeating `_guard` declares a merge even though no later block captures it.
/// The selected guard stays alive through common work and drops at flow exit.
#[kaalang]
fn unused_guard_survives_a_merge(condition: bool, cell: &RefCell<usize>) -> bool {
    #[question("Which branch borrows the cell?")]
    |condition| -> (yes, no) { condition };

    #[action("Borrow the cell on the yes branch.")]
    |yes, cell| -> _guard { cell.borrow_mut() };

    #[action("Borrow the cell on the no branch.")]
    |no, cell| -> _guard { cell.borrow_mut() };

    #[action("Check that the merged guard is still alive.")]
    |cell| -> result { cell.try_borrow_mut().is_err() };
}

#[test]
fn an_uncaptured_merged_guard_lives_until_flow_exit() {
    for condition in [true, false] {
        let cell = RefCell::new(3);
        assert!(unused_guard_survives_a_merge(condition, &cell));
        assert!(cell.try_borrow_mut().is_ok());
    }
}
