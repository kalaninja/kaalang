use std::cell::RefCell;

use kaalang::kaa;

/// An explicit value capture may release a branch-local guard before its scope
/// ends. The other branch never creates or drops a guard.
#[kaa]
fn drop_a_branch_guard(condition: bool, cell: &RefCell<usize>) -> usize {
    #[kaa::question("Measure the cell?")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Borrow the cell exclusively.")]
    let guard = |yes, cell| cell.borrow_mut();

    #[kaa::action("Read the guard.")]
    let size = |&guard| **guard;

    #[kaa::action("Release the guard before the merge.")]
    |guard| {
        drop(guard);
    };

    #[kaa::action("Use the fallback size.")]
    let size = |no| 8;

    #[kaa::action("Mutate the cell after the merge.")]
    let end = |size, cell| {
        *cell.borrow_mut() += 1;
        size
    };

    |end| return end;
}

#[test]
fn the_explicit_drop_releases_the_borrow_before_common_work() {
    for (condition, expected) in [(true, 3), (false, 8)] {
        let cell = RefCell::new(3);
        assert_eq!(drop_a_branch_guard(condition, &cell), expected);
        assert_eq!(*cell.borrow(), 4);
    }
}
