//! A do-while loop: the question comes last, so the work above it always runs
//! at least once. Every number has a last digit, zero included.

use kaalang::kaa;

#[kaa]
pub(super) fn do_while(mut number: u64) -> Vec<u8> {
    #[kaa::action("📭 Start an empty list for the digits.")]
    let mut digits = || Vec::new();

    #[kaa::cycle("Collect the number's decimal digits from right to left.")]
    let collected = loop {
        #[kaa::action("✂️ Append the last digit to the list; remove it from the number.")]
        |&mut number, &mut digits| {
            digits.push((*number % 10) as u8);
            *number /= 10;
        };

        #[kaa::question("Have all digits been removed from the number?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (collected, again) = |number| number == 0;

        |again| continue;
    };

    #[kaa::action("🔄 Reverse the collected digits to read from left to right.")]
    let result = |collected, mut digits| {
        digits.reverse();
        digits
    };

    |result| return result;
}
