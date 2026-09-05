use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn conjunction_effect(left: bool, right: bool, calls: &Cell<[u8; 2]>, effects: &Cell<u8>) {
    #[question("Enable the left input.")]
    |left, &calls| -> (a, _left_no) {
        let mut counts = calls.get();
        counts[0] += 1;
        calls.set(counts);
        left
    };

    #[question("Enable the right input.")]
    |right, &calls| -> (b, _right_no) {
        let mut counts = calls.get();
        counts[1] += 1;
        calls.set(counts);
        right
    };

    #[action("Run when both inputs are enabled.")]
    |a, b, effects| -> () { effects.set(effects.get() + 1) };

    #[end]
    || {};
}

#[test]
fn both_questions_finish_even_when_the_effect_is_inactive() {
    for left in [false, true] {
        for right in [false, true] {
            let calls = Cell::new([0, 0]);
            let effects = Cell::new(0);
            conjunction_effect(left, right, &calls, &effects);
            assert_eq!(calls.get(), [1, 1]);
            assert_eq!(effects.get(), u8::from(left && right));
        }
    }
}
