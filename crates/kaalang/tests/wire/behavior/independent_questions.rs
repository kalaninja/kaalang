use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn independent_questions(left: bool, right: bool, calls: &Cell<u8>) -> u8 {
    #[question("Choose the left path.")]
    |left, &calls| -> (a, b) {
        calls.set(calls.get() + 1);
        left
    };

    #[question("Choose the right value.")]
    |right, &calls| -> (x, y) {
        calls.set(calls.get() + 1);
        right
    };

    #[action("Build the first right value.")]
    |x| -> value { 10u8 };

    #[action("Build the second right value.")]
    |y| -> value { 20u8 };

    #[action("Use the left path and the right value.")]
    |a, value| -> result { value + 1 };

    #[action("Use the other left path and the right value.")]
    |b, value| -> result { value + 2 };
}

#[test]
fn independent_questions_use_an_order_that_shares_their_bodies() {
    for (left, right, expected) in [
        (true, true, 11),
        (true, false, 21),
        (false, true, 12),
        (false, false, 22),
    ] {
        let calls = Cell::new(0);
        assert_eq!(independent_questions(left, right, &calls), expected);
        assert_eq!(calls.get(), 2);
    }
}
