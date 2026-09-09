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
    |source| -> (first, second, fallback) {
        match source {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Borrow the cell on the first branch.")]
    |first, cell| -> (_guard, ready) { (cell.borrow_mut(), ()) };

    #[action("Borrow the cell on the second branch.")]
    |second, cell| -> (_guard, ready) { (cell.borrow_mut(), ()) };

    #[action("Check that the partially merged guard is alive.")]
    |ready, cell| -> borrowed { cell.try_borrow_mut().is_err() };

    #[action("Continue without borrowing the cell.")]
    |fallback| -> borrowed { false };

    #[action("Check that the wider merge releases the partial guard.")]
    |borrowed, cell| -> result {
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
