use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn three_independent_questions(
    first: bool,
    second: bool,
    third: bool,
    calls: &Cell<[u8; 3]>,
    effects: &Cell<[u8; 2]>,
) {
    #[question("Enable the shared input.")]
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

    #[action("Combine the first and second inputs.")]
    |&a, b, &effects| -> () {
        let mut counts = effects.get();
        counts[0] += 1;
        effects.set(counts);
    };

    #[action("Combine the first and third inputs.")]
    |&a, c, &effects| -> () {
        let mut counts = effects.get();
        counts[1] += 1;
        effects.set(counts);
    };

    #[end]
    || {};
}

#[test]
fn independent_selections_share_a_borrowed_output() {
    for first in [false, true] {
        for second in [false, true] {
            for third in [false, true] {
                let calls = Cell::new([0, 0, 0]);
                let effects = Cell::new([0, 0]);
                three_independent_questions(first, second, third, &calls, &effects);
                assert_eq!(calls.get(), [1, 1, 1]);
                assert_eq!(
                    effects.get(),
                    [u8::from(first && second), u8::from(first && third)]
                );
            }
        }
    }
}
