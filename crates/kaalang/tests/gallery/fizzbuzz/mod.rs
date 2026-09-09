use kaalang::kaalang;

#[kaalang]
fn fizzbuzz(number: u32) -> String {
    #[choice("Which of three and five divide the number?")]
    #[case("Both three and five divide it.")]
    #[case("Only three divides it.")]
    #[case("Only five divides it.")]
    #[case("Neither divides it.")]
    let (fizz_buzz, fizz, buzz, plain) = |number| match (number % 3, number % 5) {
        (0, 0) => (),
        (0, _) => (),
        (_, 0) => (),
        _ => number,
    };

    #[action("Say FizzBuzz.")]
    let result = |fizz_buzz| String::from("FizzBuzz");

    #[action("Say Fizz.")]
    let result = |fizz| String::from("Fizz");

    #[action("Say Buzz.")]
    let result = |buzz| String::from("Buzz");

    #[action("Say the number itself.")]
    let result = |plain| plain.to_string();
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
