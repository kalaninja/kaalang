//! A while loop: the question comes first, so the work below it may never run.
//! Euclid's algorithm replaces the pair with a smaller one that shares its
//! divisors, until the second number is gone.

use kaalang::kaa;

#[kaa]
pub(super) fn while_loop(mut a: u64, mut b: u64) -> u64 {
    #[kaa::cycle("Reduce the pair to its greatest common divisor.")]
    let divisor = loop {
        #[kaa::question("Is the second number zero?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (done, again) = |b| b == 0;

        #[kaa::action("🟰 Use the first number as the greatest common divisor.")]
        let divisor = |done, a| a;

        #[kaa::action("➗ Divide the first number by the second; keep the remainder.")]
        let remainder = |again, a, b| a % b;

        #[kaa::action("Replace the pair with the second number and the remainder.")]
        |remainder, &mut a, &mut b| {
            (*a, *b) = (*b, remainder);
        };

        |again| continue;
    };

    |divisor| return divisor;
}
