use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn cartesian_choices(left: bool, right: bool, seed: u8, calls: &Cell<[u8; 2]>) -> u8 {
    #[choice("Choose the left closure.")]
    #[case("Add ten.")]
    #[case("Add twenty.")]
    |left, &seed, &calls| -> (a, b) {
        match {
            let mut counts = calls.get();
            counts[0] += 1;
            calls.set(counts);
            left
        } {
            true => move || *seed + 10,
            false => move || *seed + 20,
        }
    };

    #[choice("Choose the right closure.")]
    #[case("Add one.")]
    #[case("Add two.")]
    |right, &seed, &calls| -> (c, d) {
        match {
            let mut counts = calls.get();
            counts[1] += 1;
            calls.set(counts);
            right
        } {
            true => move || *seed + 1,
            false => move || *seed + 2,
        }
    };

    #[action("Combine the first pair of closures.")]
    |a, c| -> result { a() + c() };

    #[action("Combine the first left and second right closures.")]
    |a, d| -> result { a() + d() };

    #[action("Combine the second left and first right closures.")]
    |b, c| -> result { b() + c() };

    #[action("Combine the second pair of closures.")]
    |b, d| -> result { b() + d() };

    #[end]
    |result| {};
}

#[test]
fn independent_choices_preserve_their_anonymous_payloads() {
    for (left, right, expected) in [
        (true, true, 21),
        (true, false, 22),
        (false, true, 31),
        (false, false, 32),
    ] {
        let calls = Cell::new([0, 0]);
        assert_eq!(cartesian_choices(left, right, 5, &calls), expected);
        assert_eq!(calls.get(), [1, 1]);
    }
}
