use std::cell::RefCell;

use kaalang::kaalang;

/// Alternative guard producers transfer their value through the merge. The
/// selected guard remains alive until the common action explicitly drops it.
#[kaalang]
fn merged_guard_survives_a_merge(condition: bool, cell: &RefCell<usize>) -> usize {
    #[question("Which branch borrows the cell?")]
    |condition| -> (yes, no) { condition };

    #[action("Borrow the cell on the yes branch.")]
    |yes, cell| -> guard { cell.borrow_mut() };

    #[action("Borrow the cell on the no branch.")]
    |no, cell| -> guard { cell.borrow_mut() };

    #[action("Release the merged guard before borrowing again.")]
    |guard, cell| -> result {
        assert!(cell.try_borrow_mut().is_err());
        let size = *guard;
        drop(guard);
        *cell.borrow_mut() += 1;
        size
    };
}

#[test]
fn the_transferred_guard_keeps_its_borrow_after_the_merge() {
    for condition in [true, false] {
        let cell = RefCell::new(3);
        assert_eq!(merged_guard_survives_a_merge(condition, &cell), 3);
        assert_eq!(*cell.borrow(), 4);
    }
}
