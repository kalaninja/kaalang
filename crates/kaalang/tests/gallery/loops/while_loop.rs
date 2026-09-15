//! A while loop: the question comes first, so the work below it may never run.
//! Euclid's algorithm replaces the pair with a smaller one that shares its
//! divisors, until the second number is gone.

use kaalang::kaalang;

#[kaalang]
fn while_loop(a: u64, b: u64) -> u64 {
    #[cycle("Replace the pair with a smaller equivalent one.")]
    let divisor = |mut a, mut b| {
        #[question("Is the second number zero?")]
        #[yes("YES")]
        #[no("NO")]
        let (done, again) = |b| b == 0;

        |done, a| break a;

        #[action("➗ Divide and keep the remainder.")]
        |again, &mut a, &mut b| {
            let remainder = *a % *b;
            *a = *b;
            *b = remainder;
        };
    };

    |divisor| return divisor;
}

#[test]
fn while_loop_finds_the_greatest_common_divisor() {
    assert_eq!(while_loop(0, 0), 0);
    assert_eq!(while_loop(12, 0), 12);
    assert_eq!(while_loop(0, 12), 12);

    for (a, b, divisor) in [(12, 18, 6), (17, 5, 1), (270, 192, 6), (36, 36, 36)] {
        assert_eq!(while_loop(a, b), divisor);
        assert_eq!(while_loop(b, a), divisor);
    }
}
