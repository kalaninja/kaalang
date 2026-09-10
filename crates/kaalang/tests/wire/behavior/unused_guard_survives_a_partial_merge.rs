use std::cell::RefCell;

use kaalang::kaalang;

/// Only the first two cases merge `_guard`. It remains alive through their
/// shared continuation, then drops before the wider merge with the last case.
#[kaalang]
fn unused_guard_survives_a_partial_merge(source: u8, cell: &RefCell<usize>) -> bool {
    #[choice("Which branch borrows the cell?")]
    #[case("First borrower.")]
    #[case("Second borrower.")]
    #[case("Leave the cell unborrowed.")]
    let (first, second, fallback) = |source| match source {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Borrow the cell on the first branch.")]
    let (_guard, ready) = |first, cell| (cell.borrow_mut(), ());

    #[action("Borrow the cell on the second branch.")]
    let (_guard, ready) = |second, cell| (cell.borrow_mut(), ());

    #[action("Check that the partially merged guard is alive.")]
    let borrowed = |ready, cell| cell.try_borrow_mut().is_err();

    #[action("Continue without borrowing the cell.")]
    let borrowed = |fallback| false;

    #[action("Check that the wider merge releases the partial guard.")]
    let end = |borrowed, cell| {
        assert!(cell.try_borrow_mut().is_ok());
        borrowed
    };
}

#[test]
fn an_uncaptured_guard_stays_with_its_partial_merge_group() {
    for (source, borrowed) in [(0, true), (1, true), (2, false)] {
        let cell = RefCell::new(3);
        assert_eq!(
            unused_guard_survives_a_partial_merge(source, &cell),
            borrowed
        );
        assert!(cell.try_borrow_mut().is_ok());
    }
}
