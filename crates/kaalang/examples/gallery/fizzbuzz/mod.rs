use kaalang::kaa;

#[kaa]
fn fizzbuzz(number: u32) -> String {
    #[kaa::choice("Which of 3 and 5 divide the number?")]
    #[kaa::case("Both 3 and 5.")]
    #[kaa::case("Only 3.")]
    #[kaa::case("Only 5.")]
    #[kaa::case("Neither 3 nor 5.")]
    let (fizz_buzz, fizz, buzz, plain) = |number| match (number % 3, number % 5) {
        (0, 0) => (),
        (0, _) => (),
        (_, 0) => (),
        _ => number,
    };

    #[kaa::action("🎉 Use `FizzBuzz` for this number.")]
    let end = |fizz_buzz| String::from("FizzBuzz");

    #[kaa::action("🫧 Use `Fizz` for this number.")]
    let end = |fizz| String::from("Fizz");

    #[kaa::action("🐝 Use `Buzz` for this number.")]
    let end = |buzz| String::from("Buzz");

    #[kaa::action("🔢 Use the number itself, written as text.")]
    let end = |plain| plain.to_string();

    |end| return end;
}

pub(super) fn check() {
    let values: Vec<_> = (1..=15).map(fizzbuzz).collect();
    assert_eq!(
        values,
        [
            "1", "2", "Fizz", "4", "Buzz", "Fizz", "7", "8", "Fizz", "Buzz", "11", "Fizz", "13",
            "14", "FizzBuzz",
        ]
    );
}
