use std::cell::RefCell;

use kaalang::kaalang;

/// An explicit value capture may release a branch-local guard before its scope
/// ends. The other branch never creates or drops a guard.
#[kaalang]
fn drop_a_branch_guard(condition: bool, cell: &RefCell<usize>) -> usize {
    #[question("Measure the cell?")]
    let (yes, no) = |condition| condition;

    #[action("Borrow the cell exclusively.")]
    let guard = |yes, cell| cell.borrow_mut();

    #[action("Read the guard.")]
    let size = |&guard| **guard;

    #[action("Release the guard before the merge.")]
    |guard| {
        drop(guard);
    };

    #[action("Use the fallback size.")]
    let size = |no| 8usize;

    #[action("Mutate the cell after the merge.")]
    let end = |size, cell| {
        *cell.borrow_mut() += 1;
        size
    };
}

#[test]
fn the_explicit_drop_releases_the_borrow_before_common_work() {
    for (condition, expected) in [(true, 3), (false, 8)] {
        let cell = RefCell::new(3);
        assert_eq!(drop_a_branch_guard(condition, &cell), expected);
        assert_eq!(*cell.borrow(), 4);
    }
}
