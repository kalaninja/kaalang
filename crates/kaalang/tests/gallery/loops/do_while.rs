//! A do-while loop: the question comes last, so the work above it always runs
//! at least once. Every number has a last digit, zero included.

use kaalang::kaalang;

#[kaalang]
fn do_while(number: u64) -> Vec<u8> {
    #[action("📭 Start with no digits.")]
    let digits = || Vec::new();

    #[cycle("Take one digit off the end.")]
    let collected = |mut number, mut digits| {
        #[action("✂️ Record the last digit and drop it.")]
        |&mut number, &mut digits| {
            digits.push((*number % 10) as u8);
            *number /= 10;
        };

        #[question("Is the number gone?")]
        #[yes("YES")]
        #[no("NO")]
        let (done, _again) = |number| number == 0;

        |done, digits| break digits;
    };

    #[action("🔄 Put the digits back in reading order.")]
    let result = |mut collected| {
        collected.reverse();
        collected
    };

    |result| return result;
}

#[test]
fn do_while_records_at_least_one_digit() {
    assert_eq!(do_while(0), [0]);
    assert_eq!(do_while(7), [7]);
    assert_eq!(do_while(1024), [1, 0, 2, 4]);
}
