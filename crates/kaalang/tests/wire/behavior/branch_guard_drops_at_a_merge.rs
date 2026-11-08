use std::cell::RefCell;

use kaalang::kaa;

/// The branches share only `ready`; their differently named guards remain
/// local and drop at the merge before common work borrows the cell again.
#[kaa]
fn branch_guard_drops_at_a_merge(condition: bool, cell: &RefCell<usize>) -> usize {
    #[kaa::question("Measure the cell?")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Borrow the cell and read its size.")]
    let (_yes_guard, ready) = |yes, cell| {
        let guard = cell.borrow_mut();
        let size = *guard;
        (guard, size)
    };

    #[kaa::action("Borrow the cell and use the fallback size.")]
    let (_no_guard, ready) = |no, cell| (cell.borrow_mut(), 8);

    #[kaa::action("Mutate the cell after the merge.")]
    let end = |ready, cell| {
        *cell.borrow_mut() += 1;
        ready
    };

    |end| return end;
}

#[test]
fn the_branch_scope_releases_the_guard_before_common_work() {
    for (condition, expected) in [(true, 3), (false, 8)] {
        let cell = RefCell::new(3);
        assert_eq!(branch_guard_drops_at_a_merge(condition, &cell), expected);
        assert_eq!(*cell.borrow(), 4);
    }
}
