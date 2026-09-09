use std::cell::RefCell;

use kaalang::kaalang;

/// The branches share only `ready`; their differently named guards remain
/// local and drop at the merge before common work borrows the cell again.
#[kaalang]
fn branch_guard_drops_at_a_merge(condition: bool, cell: &RefCell<usize>) -> usize {
    #[question("Measure the cell?")]
    |condition| -> (yes, no) { condition };

    #[action("Borrow the cell and read its size.")]
    |yes, cell| -> (_yes_guard, ready) {
        let guard = cell.borrow_mut();
        let size = *guard;
        (guard, size)
    };

    #[action("Borrow the cell and use the fallback size.")]
    |no, cell| -> (_no_guard, ready) { (cell.borrow_mut(), 8usize) };

    #[action("Mutate the cell after the merge.")]
    |ready, cell| -> result {
        *cell.borrow_mut() += 1;
        ready
    };
}

#[test]
fn the_branch_scope_releases_the_guard_before_common_work() {
    for (condition, expected) in [(true, 3), (false, 8)] {
        let cell = RefCell::new(3);
        assert_eq!(branch_guard_drops_at_a_merge(condition, &cell), expected);
        assert_eq!(*cell.borrow(), 4);
    }
}
