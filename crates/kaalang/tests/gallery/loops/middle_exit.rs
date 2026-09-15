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

    #[action("🤔 Guess half the number.")]
    let mut guess = |positive, number| number / 2 + 1;

    #[cycle("Average the guess with the number divided by it.")]
    let root = |number, mut guess| {
        #[action("⚖️ Average the guess with its quotient.")]
        let next = |number, guess| u64::midpoint(guess, number / guess);

        #[question("Has the guess stopped shrinking?")]
        #[yes("YES")]
        #[no("NO")]
        let (done, again) = |next, guess| next >= guess;

        |done, guess| break guess;

        #[action("🔽 Take the smaller guess.")]
        |again, next, &mut guess| *guess = next;
    };

    #[action("⭕ The root of zero is zero.")]
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
