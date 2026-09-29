//! A loop that leaves from the middle: the next guess is computed before the
//! question and installed after it, so neither a while nor a do-while header
//! fits. Newton's method walks down to the integer square root.

use kaalang::kaalang;

#[kaalang]
fn middle_exit(number: u64) -> u64 {
    #[question("Is the number positive?")]
    #[yes("YES")]
    #[no("NO")]
    let (positive, zero) = |number| number > 0;

    #[action("🤔 Start with a guess at or above the square root.")]
    let mut guess = |positive, number| number / 2 + 1;

    #[cycle("Lower the guess until it is the integer square root.")]
    let root = |guess| {
        #[action("Divide the number by the current guess.")]
        let quotient = |number, guess| number / guess;

        #[action("⚖️ Average the guess and quotient to get a new guess.")]
        let next = |guess, quotient| u64::midpoint(guess, quotient);

        #[question("Is the new guess at least as large as the current one?")]
        #[yes("YES")]
        #[no("NO")]
        let (done, again) = |next, guess| next >= guess;

        #[action("✅ Use the current guess as the integer square root.")]
        let root = |done, guess| guess;

        #[action("🔽 Replace the current guess with the smaller new guess.")]
        |again, next, &mut guess| *guess = next;

        |again| continue;
    };

    #[action("⭕ The square root of zero is zero.")]
    let root = |zero| 0;

    |root| return root;
}

#[test]
fn middle_exit_finds_the_integer_square_root() {
    for number in 0..200 {
        assert_eq!(middle_exit(number), number.isqrt());
    }

    for number in [u64::from(u32::MAX), 1 << 40, u64::MAX] {
        assert_eq!(middle_exit(number), number.isqrt());
    }
}
