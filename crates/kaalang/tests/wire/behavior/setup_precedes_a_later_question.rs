use std::cell::Cell;

use kaalang::kaalang;

/// The question is authored before the setup, and captures nothing it produces.
/// Both of its branch actions borrow `setup`, so RFC 0001 §7 orders the setup
/// before the selection: the recorded order proves the rule rather than the
/// authored sequence.
#[kaalang]
fn setup_precedes_a_later_question(condition: bool, order: &Cell<u8>) -> u8 {
    #[question("Take the short branch?")]
    |condition, &order| -> (short, long) {
        order.set(order.get() * 10 + 2);
        condition
    };

    #[action("Prepare the shared setup.")]
    |&order| -> setup {
        order.set(order.get() * 10 + 1);
        7u8
    };

    #[action("Use the setup on the short branch.")]
    |short, &setup| -> result { setup + 1 };

    #[action("Use the setup on the long branch.")]
    |long, &setup| -> result { setup + 2 };
}

#[test]
fn the_setup_runs_before_the_question_that_selects_its_consumers() {
    for (condition, expected) in [(true, 8), (false, 9)] {
        let order = Cell::new(0);
        assert_eq!(setup_precedes_a_later_question(condition, &order), expected);
        // 1 then 2. Had the question evaluated first the record would read 21.
        assert_eq!(order.get(), 12);
    }
}
