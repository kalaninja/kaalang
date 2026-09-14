use std::cell::Cell;

use kaalang::kaalang;

/// The right-hand question merges its values before the left-hand one opens
/// its branches, so each body still runs once and in the order written.
#[kaalang]
fn independent_questions(left: bool, right: bool, calls: &Cell<u8>) -> u8 {
    #[question("Choose the right value.")]
    let (x, y) = |right, &calls| {
        calls.set(calls.get() + 1);
        right
    };

    #[action("Build the first right value.")]
    let value = |x| 10;

    #[action("Build the second right value.")]
    let value = |y| 20;

    #[question("Choose the left branch.")]
    let (a, b) = |left, &calls| {
        calls.set(calls.get() + 1);
        left
    };

    #[action("Use the left branch and the right value.")]
    let end = |a, value| value + 1;

    #[action("Use the other left branch and the right value.")]
    let end = |b, value| value + 2;

    |end| return end;
}

#[test]
fn successive_questions_run_in_source_order() {
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
