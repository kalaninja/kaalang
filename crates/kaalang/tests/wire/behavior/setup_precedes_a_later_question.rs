use std::cell::Cell;

use kaalang::kaalang;

/// The setup belongs to no branch, so it is written above the question that
/// selects its consumers. The recorded order is the source order.
#[kaalang]
fn setup_precedes_a_later_question(condition: bool, order: &Cell<u8>) -> u8 {
    #[action("Prepare the shared setup.")]
    |&order| -> setup {
        order.set(order.get() * 10 + 1);
        7u8
    };

    #[question("Take the short branch?")]
    |condition, &order| -> (short, long) {
        order.set(order.get() * 10 + 2);
        condition
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
        // 1 then 2, exactly as the two blocks are written.
        assert_eq!(order.get(), 12);
    }
}
