use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn triangle_questions(
    first: bool,
    second: bool,
    third: bool,
    calls: &Cell<[u8; 3]>,
    effects: &Cell<[u8; 3]>,
) {
    #[question("Enable the first input.")]
    |first, &calls| -> (a, _first_no) {
        let mut counts = calls.get();
        counts[0] += 1;
        calls.set(counts);
        first
    };

    #[question("Enable the second input.")]
    |second, &calls| -> (b, _second_no) {
        let mut counts = calls.get();
        counts[1] += 1;
        calls.set(counts);
        second
    };

    #[question("Enable the third input.")]
    |third, &calls| -> (c, _third_no) {
        let mut counts = calls.get();
        counts[2] += 1;
        calls.set(counts);
        third
    };

    #[action("Use the first and second inputs.")]
    |&a, &b, &effects| -> () {
        let mut counts = effects.get();
        counts[0] += 1;
        effects.set(counts);
    };

    #[action("Use the second and third inputs.")]
    |&b, &c, &effects| -> () {
        let mut counts = effects.get();
        counts[1] += 1;
        effects.set(counts);
    };

    #[action("Use the first and third inputs.")]
    |&a, &c, &effects| -> () {
        let mut counts = effects.get();
        counts[2] += 1;
        effects.set(counts);
    };

    #[end]
    || {};
}

#[test]
fn every_active_question_pair_runs_once() {
    for first in [false, true] {
        for second in [false, true] {
            for third in [false, true] {
                let calls = Cell::new([0, 0, 0]);
                let effects = Cell::new([0, 0, 0]);
                triangle_questions(first, second, third, &calls, &effects);
                assert_eq!(calls.get(), [1, 1, 1]);
                assert_eq!(
                    effects.get(),
                    [
                        u8::from(first && second),
                        u8::from(second && third),
                        u8::from(first && third),
                    ]
                );
            }
        }
    }
}
