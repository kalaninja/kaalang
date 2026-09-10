use std::cell::Cell;

use kaalang::kaalang;

/// The setup belongs to no branch, so it is written above the question that
/// selects its consumers. The recorded order is the source order.
#[kaalang]
fn setup_precedes_a_later_question(condition: bool, order: &Cell<u8>) -> u8 {
    #[action("Prepare the shared setup.")]
    let setup = |&order| {
        order.set(order.get() * 10 + 1);
        7
    };

    #[question("Take the short branch?")]
    let (short, long) = |condition, &order| {
        order.set(order.get() * 10 + 2);
        condition
    };

    #[action("Use the setup on the short branch.")]
    let end = |short, &setup| setup + 1;

    #[action("Use the setup on the long branch.")]
    let end = |long, &setup| setup + 2;
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
