use kaalang::kaalang;

#[kaalang]
fn fizzbuzz(number: u32) -> String {
    #[choice("Which of 3 and 5 divide the number?")]
    #[case("Both.")]
    #[case("Only 3.")]
    #[case("Only 5.")]
    #[case("Neither.")]
    let (fizz_buzz, fizz, buzz, plain) = |number| match (number % 3, number % 5) {
        (0, 0) => (),
        (0, _) => (),
        (_, 0) => (),
        _ => number,
    };

    #[action("🎉 Say FizzBuzz.")]
    let end = |fizz_buzz| String::from("FizzBuzz");

    #[action("🫧 Say Fizz.")]
    let end = |fizz| String::from("Fizz");

    #[action("🐝 Say Buzz.")]
    let end = |buzz| String::from("Buzz");

    #[action("🔢 Say the number.")]
    let end = |plain| plain.to_string();

    |end| return end;
}

#[test]
fn fizzbuzz_names_the_first_fifteen_numbers() {
    let said: Vec<String> = (1..=15).map(fizzbuzz).collect();

    assert_eq!(
        said,
        [
            "1", "2", "Fizz", "4", "Buzz", "Fizz", "7", "8", "Fizz", "Buzz", "11", "Fizz", "13",
            "14", "FizzBuzz"
        ]
    );
}
