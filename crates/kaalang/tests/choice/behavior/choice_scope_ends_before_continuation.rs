use std::cell::{Cell, RefCell};

use kaalang::kaalang;

/// Both the captured input guard and the match binding leave scope before the
/// selected continuation mutably borrows the cell. Case values have different
/// types, and evaluating the scrutinee records exactly one visit.
#[kaalang]
fn choice_scope_ends_before_continuation(
    condition: bool,
    cell: &RefCell<usize>,
    evaluations: &Cell<usize>,
) -> usize {
    #[action("Borrow the cell before choosing.")]
    |cell| -> input_guard { cell.borrow() };

    #[choice("Which increment should be applied?")]
    #[case("Use the next value.")]
    #[case("Use the fallback word's length.")]
    |condition, input_guard, cell, evaluations| -> (number, text) {
        match {
            evaluations.set(evaluations.get() + 1);
            (condition, cell.borrow())
        } {
            (true, selected) => {
                assert_eq!(*selected, *input_guard);
                *selected + 1
            }
            (false, selected) => {
                assert_eq!(*selected, *input_guard);
                "fallback"
            }
        }
    };

    #[action("Apply the numeric increment after leaving the choice.")]
    |number, cell| -> result {
        let mut value = cell.borrow_mut();
        *value += number;
        *value
    };

    #[action("Apply the text length after leaving the choice.")]
    |text, cell| -> result {
        let mut value = cell.borrow_mut();
        *value += text.len();
        *value
    };
}

#[test]
fn choice_locals_drop_before_the_selected_continuation() {
    for (condition, expected) in [(true, 7), (false, 11)] {
        let cell = RefCell::new(3);
        let evaluations = Cell::new(0);
        assert_eq!(
            choice_scope_ends_before_continuation(condition, &cell, &evaluations),
            expected
        );
        assert_eq!(*cell.borrow(), expected);
        assert_eq!(evaluations.get(), 1);
    }
}
