use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn cartesian_questions(left: bool, right: bool, calls: &Cell<[u8; 2]>) -> u8 {
    #[question("Choose the left branch.")]
    |left, &calls| -> (a, b) {
        let mut counts = calls.get();
        counts[0] += 1;
        calls.set(counts);
        left
    };

    #[question("Choose the right branch.")]
    |right, &calls| -> (c, d) {
        let mut counts = calls.get();
        counts[1] += 1;
        calls.set(counts);
        right
    };

    #[action("Combine both yes branches.")]
    |a, c| -> result { 0 };

    #[action("Combine yes and no.")]
    |a, d| -> result { 1 };

    #[action("Combine no and yes.")]
    |b, c| -> result { 2 };

    #[action("Combine both no branches.")]
    |b, d| -> result { 3 };

    #[end]
    |result| {};
}

#[test]
fn every_combination_evaluates_each_question_once() {
    for (left, right, expected) in [
        (true, true, 0),
        (true, false, 1),
        (false, true, 2),
        (false, false, 3),
    ] {
        let calls = Cell::new([0, 0]);
        assert_eq!(cartesian_questions(left, right, &calls), expected);
        assert_eq!(calls.get(), [1, 1]);
    }
}
