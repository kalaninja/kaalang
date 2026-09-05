use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn triangle_choices(
    first: bool,
    second: bool,
    third: bool,
    calls: &Cell<[u8; 3]>,
    effects: &Cell<[u8; 3]>,
) {
    #[choice("Build the first payload.")]
    #[case("Provide the first string.")]
    #[case("Leave the first input inactive.")]
    |first, &calls| -> (a, _first_no) {
        match {
            let mut counts = calls.get();
            counts[0] += 1;
            calls.set(counts);
            first
        } {
            true => String::from("a"),
            false => (),
        }
    };

    #[choice("Build the second payload.")]
    #[case("Provide the second string.")]
    #[case("Leave the second input inactive.")]
    |second, &calls| -> (b, _second_no) {
        match {
            let mut counts = calls.get();
            counts[1] += 1;
            calls.set(counts);
            second
        } {
            true => String::from("b"),
            false => (),
        }
    };

    #[choice("Build the third payload.")]
    #[case("Provide the third string.")]
    #[case("Leave the third input inactive.")]
    |third, &calls| -> (c, _third_no) {
        match {
            let mut counts = calls.get();
            counts[2] += 1;
            calls.set(counts);
            third
        } {
            true => String::from("c"),
            false => (),
        }
    };

    #[action("Borrow the first and second strings.")]
    |&a, &b, &effects| -> () {
        assert_eq!(a, "a");
        assert_eq!(b, "b");
        let mut counts = effects.get();
        counts[0] += 1;
        effects.set(counts);
    };

    #[action("Borrow the second and third strings.")]
    |&b, &c, &effects| -> () {
        assert_eq!(b, "b");
        assert_eq!(c, "c");
        let mut counts = effects.get();
        counts[1] += 1;
        effects.set(counts);
    };

    #[action("Borrow the first and third strings.")]
    |&a, &c, &effects| -> () {
        assert_eq!(a, "a");
        assert_eq!(c, "c");
        let mut counts = effects.get();
        counts[2] += 1;
        effects.set(counts);
    };

    #[end]
    || {};
}

#[test]
fn every_active_choice_pair_borrows_its_payloads_once() {
    for first in [false, true] {
        for second in [false, true] {
            for third in [false, true] {
                let calls = Cell::new([0, 0, 0]);
                let effects = Cell::new([0, 0, 0]);
                triangle_choices(first, second, third, &calls, &effects);
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
