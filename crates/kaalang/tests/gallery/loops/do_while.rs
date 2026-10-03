//! A do-while loop: the question comes last, so the work above it always runs
//! at least once. Every number has a last digit, zero included.

use kaalang::kaalang;

#[kaalang]
fn do_while(mut number: u64) -> Vec<u8> {
    #[action("📭 Start an empty list for the digits.")]
    let mut digits = || Vec::new();

    #[cycle("Collect the number's decimal digits from right to left.")]
    let collected = loop {
        #[action("✂️ Append the last digit to the list; remove it from the number.")]
        |&mut number, &mut digits| {
            digits.push((*number % 10) as u8);
            *number /= 10;
        };

        #[question("Have all digits been removed from the number?")]
        #[yes("YES")]
        #[no("NO")]
        let (collected, again) = |number| number == 0;

        |again| continue;
    };

    #[action("🔄 Reverse the collected digits to read from left to right.")]
    let result = |collected, mut digits| {
        digits.reverse();
        digits
    };

    |result| return result;
}

#[test]
fn do_while_records_at_least_one_digit() {
    assert_eq!(do_while(0), [0]);
    assert_eq!(do_while(7), [7]);
    assert_eq!(do_while(1024), [1, 0, 2, 4]);
}
